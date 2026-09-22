//! Localised strings the way a C++ application has them: one compiled resource file per
//! language, the nearest one opened once, each string read when asked for.
//!
//! The translations are `locales/*.toml` beside `Cargo.toml`; `symdev build` compiles
//! each into `localedemo_strings.rsc` / `.r02` / `.r93` and installs them all, and
//! `symbian_std::strings!()` below turns `default.toml`'s keys into constants.
//!
//! What is measured, against the C++ baseline (`docs/research/cpp-parity/locale`,
//! `User::AllocSize` either side of the same steps — the file session is already open
//! on both sides before the first count):
//!
//! | step | C++ |
//! |---|---|
//! | open the nearest file, held | +4 cells, +208 bytes |
//! | one 14-character string, held | +1 cell, +36 bytes |
//! | string freed | back to the open file |
//!
//! and that the strings are the ones for the language the device reports.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::locale::Language;
use symbian_std::test_report::{Report, alloc_size, detail};

symbian_std::strings!();

const DIR: &str = "E:\\symdev\\locale";
const NOTES: &str = "E:\\symdev\\locale\\measured.txt";

/// The greeting a device set to `language` must get from these files, by the rule
/// `NearestLanguageFile` applies to them: its own file if there is one, else the
/// default. (The dialect and downgrade steps are the platform's and are not exercised:
/// this ROM offers none of them.)
fn expected(language: Language) -> &'static str {
    match language.code() {
        2 => "Bonjour depuis Rust",
        93 => "Привіт з Rust",
        _ => "Hello from Rust",
    }
}

/// The heap either side of the first read, of holding a string and of dropping it — no
/// report call in between, because recording a case allocates.
fn heap(report: &mut Report, notes: &mut String) {
    let before = alloc_size();
    let first = strings::GREETING.get();
    let held = alloc_size();
    drop(first);
    let open = alloc_size();
    let again = strings::GREETING.get().map(|t| t.len());
    let after_again = alloc_size();
    let _ = writeln!(
        notes,
        "heap_before={}/{}\nheap_open_and_held={}/{}\nheap_open={}/{}\nheap_after_second={}/{}",
        before.0, before.1, held.0, held.1, open.0, open.1, after_again.0, after_again.1
    );
    report.check("the second read works", again.is_ok());
    report.check_detail(
        "dropping the string gives its cell back",
        held.0 == open.0 + 1,
        detail!("{} -> {}", held.0, open.0),
    );
    report.check_detail(
        "a read leaves nothing behind once the file is open",
        after_again == open,
        detail!(
            "{}/{} -> {}/{}",
            open.0,
            open.1,
            after_again.0,
            after_again.1
        ),
    );
    report.check_detail(
        "the open file holds no more than C++'s 4 cells",
        open.0.saturating_sub(before.0) <= 4,
        detail!("+{} cells, +{} bytes", open.0 - before.0, open.1 - before.1),
    );
}

/// Every string, in the language the device reports.
fn table(report: &mut Report, notes: &mut String) {
    let language = Language::current();
    let _ = writeln!(notes, "user_language={}", language.code());
    for (name, key) in [
        ("GREETING", strings::GREETING),
        ("LANGUAGE_IS", strings::LANGUAGE_IS),
        ("OK", strings::OK),
    ] {
        match key.get() {
            Ok(text) => {
                let _ = writeln!(notes, "{name}={}", &*text);
            }
            Err(_) => report.fail(name, "could not be read"),
        }
    }
    match strings::GREETING.get() {
        Ok(text) => report.check_detail(
            "the greeting is the device language's",
            &*text == expected(language),
            detail!("language {}", language.code()),
        ),
        Err(_) => report.fail("the greeting is the device language's", "not read"),
    }
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("locale");
    let mut notes = String::new();
    // The file session first, as the C++ side has its `RFs` connected before it counts.
    report.checked("the notes directory", fs::create_dir_all(DIR));
    heap(&mut report, &mut notes);
    table(&mut report, &mut notes);
    report.checked(
        "the measurements are written out",
        fs::write(NOTES, notes.as_bytes()),
    );
    Ok(if report.finish()? { 0 } else { 1 })
}
