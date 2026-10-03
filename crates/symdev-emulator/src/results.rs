//! The result protocol: what an example writes on the emulated device and what
//! `symdev test --emulator` reads back (design spec §9, §11).
//!
//! The device side is `symbian_std::test_report`, which writes
//! `E:\symdev\results\<uid3>.json`. The host sees drive E: as a directory tree under
//! EKA2L1's data directory, so the same file is
//! `<data>/drives/e/symdev/results/<uid3>.json`.
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use symdev_core::{Error, Result};

use crate::json::Json;

/// The schema version `symbian_std::test_report` writes. A file from a newer writer
/// is refused rather than half-understood.
pub const SCHEMA: i64 = 1;

/// EKA2L1's data folder: a profile's own, or the user's named by `SYMDEV_EKA2L1_DATA`.
/// The drives are one level further in, under `data/` — verified against the tree on this
/// host, not assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmulatorData {
    root: PathBuf,
}

impl EmulatorData {
    pub fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// The data folder itself: `config.yml` and `data/` are in it.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<data>/data/drives/e`: the host side of the emulated drive E:, as the tree on
    /// this host really is — `~/.local/share/EKA2L1/data/drives/e/` holds `sys/bin`,
    /// `private` and everything a SIS installs.
    /// `<data>/data/drives`: every emulated drive.
    pub fn drives(&self) -> PathBuf {
        self.root.join("data").join("drives")
    }

    pub fn drive_e(&self) -> PathBuf {
        self.root.join("data").join("drives").join("e")
    }

    /// Where the example's report lands, the host spelling of
    /// `E:\symdev\results\<uid3>.json`.
    pub fn result_file(&self, uid3: u32) -> PathBuf {
        self.drive_e()
            .join("symdev")
            .join("results")
            .join(format!("{uid3:08x}.json"))
    }
}

/// One case in a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    /// `None` for a finished case; set by `symbian-test` for one listed but not finished.
    pub state: Option<CaseState>,
}

/// Where `symbian-test` got with a case: listed before the first ran, or running now. A
/// panic ends the process, so the case left `Running` is the one that panicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseState {
    Pending,
    Running,
}

/// A parsed result file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestReport {
    pub app: String,
    pub uid3: String,
    pub cases: Vec<TestCase>,
}

impl TestReport {
    /// Parses a report, refusing anything it does not fully understand: a wrong schema,
    /// a missing field, a `cases` entry that is not an object. A half-written file from
    /// a process that died mid-write must not read as a pass.
    pub fn parse(text: &str) -> Result<Self> {
        let json = Json::parse(text)?;
        let schema = json
            .get("schema")
            .and_then(Json::as_i64)
            .ok_or_else(|| Error::Other("test result file: no `schema`".into()))?;
        if schema != SCHEMA {
            return Err(Error::Other(format!(
                "test result file: schema {schema}, but this symdev reads {SCHEMA}"
            )));
        }
        let field = |name: &str| -> Result<String> {
            json.get(name)
                .and_then(Json::as_str)
                .map(str::to_owned)
                .ok_or_else(|| Error::Other(format!("test result file: no `{name}`")))
        };
        let items = json
            .get("cases")
            .and_then(Json::as_array)
            .ok_or_else(|| Error::Other("test result file: no `cases` array".into()))?;
        let mut cases = Vec::with_capacity(items.len());
        for item in items {
            let name = item
                .get("name")
                .and_then(Json::as_str)
                .ok_or_else(|| Error::Other("test result file: a case has no `name`".into()))?;
            let ok = item
                .get("ok")
                .and_then(Json::as_bool)
                .ok_or_else(|| Error::Other("test result file: a case has no `ok`".into()))?;
            let state = match item.get("state").map(|s| s.as_str()) {
                None => None,
                Some(Some("pending")) => Some(CaseState::Pending),
                Some(Some("running")) => Some(CaseState::Running),
                Some(other) => {
                    return Err(Error::Other(format!(
                        "test result file: case `{name}` has the state {}, which this symdev \
                         does not know",
                        other.unwrap_or("(not a string)")
                    )));
                }
            };
            cases.push(TestCase {
                name: name.to_owned(),
                ok,
                state,
                detail: item
                    .get("detail")
                    .and_then(Json::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        }
        Ok(Self {
            app: field("app")?,
            uid3: field("uid3")?,
            cases,
        })
    }

    /// Reads and parses the report at `path`.
    pub fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::Other(format!("read {}: {e}", path.display())))?;
        Self::parse(&text)
    }

    pub fn passed(&self) -> usize {
        self.cases.iter().filter(|c| c.ok).count()
    }

    pub fn failed(&self) -> usize {
        self.cases.len() - self.passed()
    }

    /// Whether the run passed.
    ///
    /// A report with **no cases at all** is a failure, not a vacuous pass: it is what a
    /// program that died before it tested anything leaves behind.
    pub fn is_pass(&self) -> bool {
        self.failed() == 0 && !self.cases.is_empty()
    }
}

/// Waits for `path` to appear and parse, giving up after `timeout`.
///
/// The file is polled rather than watched because it is written by another process
/// into a directory tree the emulator owns, and a parse that fails is retried until the
/// deadline: the writer is not atomic, so the first look can catch half a document.
pub fn await_report(path: &Path, timeout: Duration) -> Result<TestReport> {
    let deadline = Instant::now() + timeout;
    let mut last: Option<Error> = None;
    loop {
        if path.is_file() {
            match TestReport::read(path) {
                Ok(report) => return Ok(report),
                Err(e) => last = Some(e),
            }
        }
        if Instant::now() >= deadline {
            return Err(match last {
                Some(e) => Error::Other(format!(
                    "no readable test result at {} after {}s: {e}",
                    path.display(),
                    timeout.as_secs()
                )),
                None => Error::Other(format!(
                    "no test result at {} after {}s: the application never wrote one",
                    path.display(),
                    timeout.as_secs()
                )),
            });
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests;
