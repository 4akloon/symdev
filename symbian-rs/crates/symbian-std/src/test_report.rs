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
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;
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

    /// Records a case with a formatted detail, whether it passed or failed.
    ///
    /// The detail is kept for a passing case too, because a count is worth reading
    /// when it is right as well as when it is wrong: `4000 of 4000` in the report is
    /// what says the run actually did the work.
    pub fn check_detail(&mut self, name: &str, ok: bool, detail: core::fmt::Arguments<'_>) {
        let mut text = String::new();
        // A formatter that runs out of room is nothing to abort a test run over.
        let _ = text.write_fmt(detail);
        self.record(name, ok, &text);
    }

    /// Records a case that failed, with what went wrong.
    pub fn fail(&mut self, name: &str, detail: &str) {
        self.record(name, false, detail);
    }

    /// Records a case from a `Result`, and hands the value back so the example can go
    /// on using it.
    ///
    /// A failure is recorded with the error's `Debug` text, which for
    /// [`crate::io::Error`] is the `std` kind and the `e32err.h` name and code.
    pub fn checked<T, E: core::fmt::Debug>(
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
                let mut detail = String::new();
                // A formatter that runs out of room is nothing to abort a test run
                // over: the case is already recorded as a failure either way.
                let _ = write!(detail, "{e:?}");
                self.record(name, false, &detail);
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
        // `write!` to a `String` cannot fail; the result is discarded rather than
        // unwrapped, which this SDK does not allow outside tests.
        let _ = write!(path, "\\{:08x}.json", self.uid3);
        path
    }

    /// The JSON document, exactly as it goes into the file.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "{{\"schema\":{SCHEMA},\"app\":\"");
        escape_into(&mut out, &self.app);
        let _ = write!(
            out,
            "\",\"uid3\":\"0x{:08x}\",\"passed\":{},\"failed\":{},\"cases\":[",
            self.uid3,
            self.passed(),
            self.failed()
        );
        for (i, case) in self.cases.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str("{\"name\":\"");
            escape_into(&mut out, &case.name);
            let _ = write!(out, "\",\"ok\":{}", case.ok);
            if !case.detail.is_empty() {
                out.push_str(",\"detail\":\"");
                escape_into(&mut out, &case.detail);
                out.push('"');
            }
            out.push('}');
        }
        out.push_str("]}");
        out
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

/// Appends `text` to `out` as the body of a JSON string.
///
/// Escapes what RFC 8259 requires — the quote, the backslash and everything below
/// `0x20` — and passes the rest through, since the file is UTF-8 and so is a Rust
/// `str`.
fn escape_into(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
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
