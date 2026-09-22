//! Translations for the strings the application itself shows, as C++ ships them.
//!
//! One file per language beside `Cargo.toml` — `locales/default.toml` is the fallback,
//! `locales/<language>.toml` a translation, one `key = "text"` per line:
//!
//! ```toml
//! # locales/ukrainian.toml
//! greeting = "Привіт з Rust"
//! ```
//!
//! and in the source:
//!
//! ```ignore
//! symbian_std::strings!();
//!
//! let greeting = strings::GREETING.get()?;
//! ui.note_info(&greeting);
//! ```
//!
//! # What it costs, and why this and not a table in the image
//!
//! `symdev build` compiles each file into `<app>_strings.rsc` / `.r<code>` and installs
//! them all. At run time the program opens the one `BaflUtils::NearestLanguageFile`
//! picks for the device, once, and reads a string with one `RFile::Read` when asked — the
//! C++ model, measured against it in EKA2L1: the open file holds 1 heap cell (its index;
//! C++'s `RResourceFile` holds 4), a string held is 1 more, and dropping it gives that
//! back. No C++ and no `TRAP` is involved: symdev writes these files, and the reader
//! understands exactly what it writes (experiment 102). No other language
//! is ever in memory. A table compiled into the executable holds every language for the
//! life of the process, which is what the user ruled out
//! (`docs/research/experiment-backlog.md`, the native localisation entry).
//!
//! # What the compiler refuses
//!
//! - A key that is not in `default.toml` is not a constant: `E0425` at the use, with
//!   rustc's own "a constant with a similar name exists".
//! - A key missing from one translation, or one a translation has and the default does
//!   not: a `compile_error!` naming the file and the key.
//!
//! [`Language::current`] stays for the program that wants to know the device's language
//! itself; choosing the file does not need it.
pub use symbian_core::locale::{Language, Str, Text};
