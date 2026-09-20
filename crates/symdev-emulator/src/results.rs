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

/// EKA2L1's data directory, where the emulated drives live as host directories.
///
/// `SYMDEV_EKA2L1_DATA` overrides it; the default is the path EKA2L1 itself uses on
/// Linux, `$XDG_DATA_HOME/EKA2L1` (`~/.local/share/EKA2L1`), which is where `symdev
/// run` has been installing since experiment 48. The drives are one level further in,
/// under `data/` — verified against the tree on this host, not assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmulatorData {
    root: PathBuf,
}

impl EmulatorData {
    pub fn from_env() -> Result<Self> {
        if let Some(v) = std::env::var_os("SYMDEV_EKA2L1_DATA").filter(|v| !v.is_empty()) {
            return Ok(Self { root: v.into() });
        }
        let share = match std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            Some(v) => PathBuf::from(v),
            None => {
                let home = std::env::var_os("HOME")
                    .filter(|v| !v.is_empty())
                    .ok_or_else(|| {
                        Error::Other(
                        "cannot find EKA2L1's data directory: neither HOME nor XDG_DATA_HOME is \
                         set; set SYMDEV_EKA2L1_DATA to the directory holding drives/"
                            .into(),
                    )
                    })?;
                PathBuf::from(home).join(".local/share")
            }
        };
        Ok(Self {
            root: share.join("EKA2L1"),
        })
    }

    pub fn at(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// `<data>/data/drives/e`: the host side of the emulated drive E:, as the tree on
    /// this host really is — `~/.local/share/EKA2L1/data/drives/e/` holds `sys/bin`,
    /// `private` and everything a SIS installs.
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
            cases.push(TestCase {
                name: name.to_owned(),
                ok,
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
