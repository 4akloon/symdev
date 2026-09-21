//! The device's UI language, and the rule for choosing a translation with it.
//!
//! This is the **inside** half of localisation: strings only our own Rust ever reads.
//! The other half — the name the phone's launcher shows *without running the
//! application* — cannot work this way at all, because nothing of ours is running
//! when the launcher needs it; that one is a `localisable_resource_file` compiled per
//! language into `<app>.r01`, `<app>.r93`… and found by
//! `BaflUtils::NearestLanguageFile`, and it lives in symdev's resource generator, not
//! here.
//!
//! What an application writes is [`locale!`](crate::locale!), which is in
//! `symbian-std` because that is the crate an application already depends on. This
//! module is what the declaration it expands to is built from:
//!
//! - [`Language`] — one `TLanguage` value, and [`Language::current`], which asks
//!   `User::Language()` once and caches it.
//! - [`lang`] — every `TLanguage` the SDK names, under the word a developer writes.
//!
//! # The fallback chain
//!
//! A declaration names a set of languages and a default. For a device reporting `L`,
//! the string chosen is the first of:
//!
//! 1. `L` itself, if the declaration has it;
//! 2. [`Language::base(L)`](Language::base) — the language `L` is a **dialect** of,
//!    if the declaration has that. This step exists because a phone set to
//!    `ELangEnglish_Apac` must still read English, and it is not a guess: the `_Apac`
//!    suffix is in `e32const.h`'s own enumerator name over an `ELangEnglish` it also
//!    declares. It covers exactly the seven enumerators that carry such a suffix.
//! 3. the declared default.
//!
//! The chain stops there on purpose. The platform has a fourth step of its own —
//! `TLocale::LanguageDowngrade(0..2)` (`e32std.h` line 2278), three licensee-set
//! languages that `BaflUtils::NearestLanguageFile` walks — and it is **not**
//! implemented: reading it needs the layout of `TLocale` and a `TLocale::Refresh`
//! call, and `TODO: what TLocale::Refresh writes into iLanguageDowngrade on this ROM
//! (not observed)`. Nor does the chain treat `ELangAmerican`, `ELangCanadianEnglish`,
//! `ELangAustralian` or `ELangInternationalEnglish` as English: they are English to a
//! reader, but `e32const.h` declares no relation between them and `ELangEnglish`, so
//! an application that wants them lists them itself.
mod language;

pub mod lang;

pub use language::Language;
