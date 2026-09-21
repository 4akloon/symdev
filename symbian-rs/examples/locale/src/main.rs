//! Localisation of the strings the application itself shows (the Rust half).
//!
//! Two things are measured here and nothing is assumed:
//!
//! 1. **What `User::Language()` actually returns** on this machine. The raw
//!    `TLanguage` integer goes into `E:\symdev\locale\measured.txt` next to the name
//!    this SDK gives it, so the host can read it back without a screenshot.
//! 2. **That the chain picks what it says it picks.** Every case runs through
//!    [`Text::get_in`], which takes a language instead of asking for one, so the
//!    fallback rule is exercised for languages this device is not set to and could
//!    not be set to — a dialect (`ELangEnglish_Apac`), a language the table has
//!    (`ELangUkrainian`), and one it does not (`ELangGerman`).
//!
//! The strings themselves are in [`strings`], and the point of the whole design is in
//! that file: one row per key, one column per language, and rustc refusing a row that
//! is missing a column.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::locale::{Language, lang};
use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::Report;

mod strings;

const DIR: &str = "E:\\symdev\\locale";
const NOTES: &str = "E:\\symdev\\locale\\measured.txt";

/// What the device is set to, as the device answers it — the one number on this
/// branch that no header could have told us.
fn measure(report: &mut Report, notes: &mut String) {
    let current = Language::current();
    let _ = writeln!(notes, "user_language_raw={}", current.code());
    let _ = writeln!(
        notes,
        "user_language_name={}",
        name_of(current).unwrap_or("<not named by this SDK>")
    );
    let _ = writeln!(
        notes,
        "user_language_base={}",
        match current.base() {
            Some(base) => base.code(),
            None => 0xFFFF,
        }
    );
    // Read twice: the second call must come out of the cache, not off euser, and the
    // only thing an application can check from inside is that it is the same answer.
    report.check(
        "the language reads the same twice",
        Language::current() == current,
    );
    let _ = writeln!(notes, "greeting_here={}", strings::GREETING.get());
}

/// The fallback chain, one case per rule, run without touching the device's setting.
fn chain(report: &mut Report, notes: &mut String) {
    let cases: [(&str, Language, &str); 6] = [
        // 1. The language itself, when the table has it.
        ("ukrainian", lang::ukrainian, "Привіт з Rust"),
        ("french", lang::french, "Bonjour depuis Rust"),
        ("english", lang::english, "Hello from Rust"),
        // 2. A dialect falls back to the language it is a dialect of.
        ("english_apac", lang::english_apac, "Hello from Rust"),
        // 3. Anything else falls back to the declared default, which is english.
        ("german", lang::german, "Hello from Rust"),
        ("none", lang::none, "Hello from Rust"),
    ];
    for (name, language, expected) in cases {
        let got = strings::GREETING.get_in(language);
        let _ = writeln!(notes, "chain[{name}]={got}");
        report.check(name, got == expected);
    }
}

/// Every key, in the language the device reports, so a human looking at the file can
/// see the table came through and not just the one string the checks use.
fn table(notes: &mut String) {
    let _ = writeln!(notes, "GREETING={}", strings::GREETING.get());
    let _ = writeln!(notes, "LANGUAGE_IS={}", strings::LANGUAGE_IS.get());
    let _ = writeln!(notes, "OK={}", strings::OK.get());
}

/// The handful of `TLanguage` names this example bothers to print. A full table would
/// be 108 lines of no interest: what matters is that the raw number is written out.
fn name_of(language: Language) -> Option<&'static str> {
    let named: [(Language, &str); 8] = [
        (lang::test, "ELangTest"),
        (lang::english, "ELangEnglish"),
        (lang::french, "ELangFrench"),
        (lang::german, "ELangGerman"),
        (lang::ukrainian, "ELangUkrainian"),
        (lang::english_apac, "ELangEnglish_Apac"),
        (lang::other, "ELangOther"),
        (lang::none, "ELangNone"),
    ];
    named
        .iter()
        .find(|(candidate, _)| *candidate == language)
        .map(|(_, name)| *name)
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("locale");
    let mut notes = String::new();
    measure(&mut report, &mut notes);
    chain(&mut report, &mut notes);
    table(&mut notes);
    report.checked("the measurements are written out", write_notes(&notes));
    Ok(if report.finish()? { 0 } else { 1 })
}

fn write_notes(notes: &str) -> Result<()> {
    fs::create_dir_all(DIR)?;
    fs::write(NOTES, notes.as_bytes())
}
