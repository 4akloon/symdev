//! How an example says whether it passed, in a file `symdev test --emulator` reads
//! back off the emulated drive (design spec §11).
//!
//! The channel is a JSON file at `E:\symdev\results\<uid3>.json`, where `<uid3>` is the
//! application's UID3 as eight lowercase hex digits with no `0x`. symdev finds it at
//! `~/.local/share/EKA2L1/data/drives/e/symdev/results/<uid3>.json` after the run.
//!
//! The shape, which symdev's reader and this writer are the two halves of:
//!
//! ```json
//! {"schema":1,"app":"files","uid3":"0xe0000691","passed":2,"failed":1,
//!  "cases":[{"name":"write","ok":true},
//!           {"name":"read back","ok":false,"detail":"KErrEof (-25)"}]}
//! ```
//!
//! `passed` and `failed` are counts, and a run is a pass only when `failed` is zero
//! **and** at least one case ran: a report with no cases is a program that died before
//! it tested anything, and symdev calls that a failure.
//!
//! A deliberately failing case must come out as a failure, and that is the property the
//! harness is verified against — a harness that cannot fail is not a harness.
//!
//! On the `no_std` path none of it formats through `core::fmt` (experiment 103): a
//! detail is written by the fast [`crate::write!`] through [`detail!`], an error by its
//! [`Evidence`], and the file by plain appends. An example that formats nothing else
//! therefore carries no `core::fmt` at all, which is what lets its size be compared
//! with C++.
use alloc::string::String;
use alloc::vec::Vec;

mod evidence;
mod json;

pub use evidence::{Evidence, Hex};
/// How many cells this thread's heap holds (`User::CountAllocCells`). Counting either
/// side of a code path turns "allocates nothing" into a measured case, and it is the
/// same count a C++ test would take, so the two can be compared.
pub use symbian_core::user::{alloc_cells, alloc_size};

#[cfg(not(feature = "std"))]
use crate::fs;
#[cfg(not(feature = "std"))]
use crate::io::Result;
#[cfg(feature = "std")]
use std::fs;
#[cfg(feature = "std")]
use std::io::Result;

/// A case's detail, written with `write!`'s arguments: `detail!("{got} of {want}")`
/// is what [`Report::check_detail`] takes where it used to take `format_args!`.
///
/// It expands to a closure that runs the fast [`crate::write!`] into the detail text,
/// so a plain `{}` of a string or an integer costs no `core::fmt`; anything else — a
/// spec such as `{:x}`, a user's `Display` — is `core::write!` exactly, and links it.
/// For an error or a `Result`, write its [`Evidence`]: `detail!("{}", outcome.shown())`.
#[macro_export]
macro_rules! detail {
    ($($format:tt)+) => {
        |out: &mut _| {
            // What the fast macro's fallback calls, for a piece that is not on its list.
            use ::core::fmt::Write as _;
            // A `String` has room for anything, so there is no error to report.
            let _ = $crate::write!(out, $($format)+);
        }
    };
}

pub use crate::detail;

/// The version of the shape above, so a later change is visible to an older reader.
pub const SCHEMA: u32 = 1;

/// The directory every example writes its result into.
pub const RESULTS_DIR: &str = "E:\\symdev\\results";

/// One case: a name, whether it passed, and for a failure what went wrong. `state` is
/// `pending` or `running` for a case `symbian-test` has listed but not finished, so a
/// panic, which ends the process, leaves the file saying which case it hit.
struct Case {
    name: String,
    ok: bool,
    detail: String,
    state: Option<&'static str>,
}

/// The result of one example run, built case by case and written out at the end.
///
/// ```ignore
/// let mut report = symbian_std::report!("files");
/// report.check("write", written == BYTES.len());
/// report.check_detail("read", got == 4000, detail!("{got} of 4000"));
/// report.checked("rename", fs::rename(FROM, TO));
/// report.finish()?;
/// ```
pub struct Report {
    app: String,
    uid3: u32,
    cases: Vec<Case>,
}

impl Report {
    /// A report for `app` (a short name, only for a human reading the file) whose file is
    /// named by `uid3`. Prefer [`crate::report!`], which takes the UID3 from the
    /// application's `symdev.toml`: writing the UID a second time in the source is how it
    /// drifts, and the only symptom is `symdev test` waiting for a file nobody writes.
    pub fn with_uid3(app: &str, uid3: u32) -> Self {
        Self {
            app: String::from(app),
            uid3,
            cases: Vec::new(),
        }
    }

    /// Records a case that passed or failed on a plain condition.
    pub fn check(&mut self, name: &str, ok: bool) {
        self.record(name, ok, "");
    }

    /// Records a case with a detail, whether it passed or failed.
    ///
    /// The detail is kept for a passing case too, because a count is worth reading
    /// when it is right as well as when it is wrong: `4000 of 4000` in the report is
    /// what says the run actually did the work.
    ///
    /// `detail` writes the text; [`detail!`] makes one from `write!`'s arguments, so
    /// the call reads `report.check_detail("read", ok, detail!("{got} of {want}"))`.
    pub fn check_detail(&mut self, name: &str, ok: bool, detail: impl FnOnce(&mut String)) {
        let mut text = String::new();
        detail(&mut text);
        self.record(name, ok, &text);
    }

    /// Records a case that failed, with what went wrong.
    pub fn fail(&mut self, name: &str, detail: &str) {
        self.record(name, false, detail);
    }

    /// Records a case from a `Result`, and hands the value back so the example can go
    /// on using it.
    ///
    /// A failure is recorded with the error's [`Evidence`]: for a Symbian error, the
    /// `e32err.h` name and the `TInt`, as `KErrNotFound (-1)`.
    pub fn checked<T, E: Evidence>(
        &mut self,
        name: &str,
        outcome: core::result::Result<T, E>,
    ) -> Option<T> {
        match outcome {
            Ok(value) => {
                self.record(name, true, "");
                Some(value)
            }
            Err(e) => {
                self.record(name, false, &e.shown());
                None
            }
        }
    }

    fn record(&mut self, name: &str, ok: bool, detail: &str) {
        self.cases.push(Case {
            name: String::from(name),
            ok,
            detail: String::from(detail),
            state: None,
        });
    }

    /// Lists a case that has not run yet (`"state":"pending"`).
    pub fn pending(&mut self, name: &str) {
        self.record(name, false, "");
        if let Some(case) = self.cases.last_mut() {
            case.state = Some("pending");
        }
    }

    /// Marks a listed case as the one running now (`"state":"running"`).
    pub fn running(&mut self, name: &str) {
        if let Some(case) = self.cases.iter_mut().find(|c| c.name == name) {
            case.state = Some("running");
        }
    }

    /// Finishes a listed case: its verdict and detail, and no state.
    pub fn settle(&mut self, name: &str, ok: bool, detail: &str) {
        if let Some(case) = self.cases.iter_mut().find(|c| c.name == name) {
            case.ok = ok;
            case.detail = String::from(detail);
            case.state = None;
        }
    }

    /// How many finished cases passed.
    pub fn passed(&self) -> usize {
        self.finished().filter(|c| c.ok).count()
    }

    /// How many finished cases failed.
    pub fn failed(&self) -> usize {
        self.finished().filter(|c| !c.ok).count()
    }

    /// Whether the run passed: every case finished and passed, and there was at least one.
    pub fn is_pass(&self) -> bool {
        self.failed() == 0 && !self.cases.is_empty() && self.finished().count() == self.cases.len()
    }

    fn finished(&self) -> impl Iterator<Item = &Case> {
        self.cases.iter().filter(|c| c.state.is_none())
    }

    /// `E:\symdev\results\<uid3>.json`.
    pub fn path(&self) -> String {
        let mut path = String::from(RESULTS_DIR);
        path.push('\\');
        json::push_hex(&mut path, self.uid3, 8);
        path.push_str(".json");
        path
    }

    /// The JSON document, exactly as it goes into the file.
    pub fn to_json(&self) -> String {
        json::document(self)
    }

    /// Creates `E:\symdev\results` if it is not there and writes the report into it.
    ///
    /// Returns whether the run passed, so an example can map it to its own exit code
    /// as well as to the file.
    pub fn finish(&self) -> Result<bool> {
        self.save()?;
        Ok(self.is_pass())
    }

    /// Writes the report as it stands, creating `E:\symdev\results` if needed.
    pub fn save(&self) -> Result<()> {
        fs::create_dir_all(RESULTS_DIR)?;
        fs::write(&self.path(), self.to_json().as_bytes())
    }
}
