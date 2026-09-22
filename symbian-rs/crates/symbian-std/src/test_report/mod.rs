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
//! None of it formats through `core::fmt` (experiment 102): a detail is written by the
//! fast [`crate::write!`] through [`detail!`], an error by its [`Evidence`], and the
//! file by [`json`]'s own appends. An example that formats nothing else therefore
//! carries no `core::fmt` at all, which is what lets its size be compared with C++.
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

/// One case: a name, whether it passed, and for a failure what went wrong.
struct Case {
    name: String,
    ok: bool,
    detail: String,
}

/// The result of one example run, built case by case and written out at the end.
///
/// ```ignore
/// let mut report = Report::new("files");
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
    /// A report for `app` (a short name, only for a human reading the file), taking the
    /// application's UID3 — which names the file — from `SYMDEV_UID3`, the manifest value
    /// `symdev build` puts in cargo's environment. Prefer this to [`Report::with_uid3`]:
    /// writing the UID a second time in the source is how it drifts from `symdev.toml`,
    /// and the only symptom is `symdev test` waiting for a file nobody writes.
    pub fn new(app: &str) -> Self {
        Self::with_uid3(app, uid3_from_env())
    }

    /// A report for an application that names its own UID3.
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
        });
    }

    /// How many cases passed.
    pub fn passed(&self) -> usize {
        self.cases.iter().filter(|c| c.ok).count()
    }

    /// How many cases failed.
    pub fn failed(&self) -> usize {
        self.cases.len() - self.passed()
    }

    /// Whether the run passed: every case passed and there was at least one.
    pub fn is_pass(&self) -> bool {
        self.failed() == 0 && !self.cases.is_empty()
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
        fs::create_dir_all(RESULTS_DIR)?;
        fs::write(&self.path(), self.to_json().as_bytes())?;
        Ok(self.is_pass())
    }
}

/// `SYMDEV_UID3` as `symdev build` sets it, parsed at compile time. A build that did not
/// set it (a hand `cargo build`) gets 0, which makes the missing value obvious in the
/// file name rather than silently writing somebody else's report.
const fn uid3_from_env() -> u32 {
    match option_env!("SYMDEV_UID3") {
        Some(text) => parse_hex_u32(text.as_bytes()),
        None => 0,
    }
}

/// `0x` followed by hex digits, at compile time. Anything else is 0.
const fn parse_hex_u32(bytes: &[u8]) -> u32 {
    if bytes.len() < 3 || bytes[0] != b'0' || (bytes[1] != b'x' && bytes[1] != b'X') {
        return 0;
    }
    let mut value: u32 = 0;
    let mut i = 2;
    while i < bytes.len() {
        let digit = match bytes[i] {
            b'0'..=b'9' => bytes[i] - b'0',
            b'a'..=b'f' => bytes[i] - b'a' + 10,
            b'A'..=b'F' => bytes[i] - b'A' + 10,
            _ => return 0,
        };
        value = value * 16 + digit as u32;
        i += 1;
    }
    value
}
