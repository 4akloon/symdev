//! Translations for the strings the application itself shows.
//!
//! Nothing outside the process reads these, so they need no Symbian resource, no
//! `.rss`, no `.r93` and nothing in `symdev.toml`: they are Rust data, declared in
//! Rust, next to the code that shows them. (The *other* half of localisation — the
//! name the launcher paints under the icon, which it reads while the application is
//! **not** running — can only be a `localisable_resource_file`, and that is symdev's
//! job, not this module's.)
//!
//! [`Language`] and [`lang`] are `symbian_core::locale`, re-exported so an
//! application needs one crate; [`locale!`](crate::locale!) is the declaration.
//!
//! # Writing one
//!
//! ```ignore
//! // src/strings.rs
//! symbian_std::locale! {
//!     languages: english, ukrainian;
//!
//!     GREETING = { english: "Hello from Rust",   ukrainian: "Привіт з Rust" },
//!     CHARS    = { english: "chars",             ukrainian: "символів" },
//! }
//! ```
//!
//! ```ignore
//! // src/main.rs
//! mod strings;
//!
//! let greeting: &'static str = strings::GREETING.get();
//! ```
//!
//! The first language named is the **default**: the one a device gets when the chain
//! in [`symbian_core::locale`] runs out. [`get`](Text::get) returns a `&'static str`
//! and allocates nothing; every string is in the image's read-only data and every
//! lookup is a comparison against an integer euser was asked for once.
//!
//! # What the compiler refuses
//!
//! - **A key that does not exist.** `strings::GREETNG` is
//!   `error[E0425]: cannot find value GREETNG in module strings`, with rustc's own
//!   "a constant with a similar name exists" help.
//! - **A key missing from one language.** The expansion is a struct with one field per
//!   declared language, so a row that forgets one is
//!   `error[E0063]: missing field ukrainian in initializer of Text`. This is the whole
//!   reason the table is key-major and lives in Rust: rustc already refuses an
//!   incomplete struct literal, so there is no completeness checker to write and none
//!   to get wrong.
//! - **A language nobody declared, on one row.** `error[E0560]: struct Text has no
//!   field named french`.
//! - **A language that is not a `TLanguage`.** `languages: english, ukranian;` is
//!   `error[E0425]: cannot find value ukranian in module symbian_core::locale::lang`,
//!   again with the "did you mean" help, because the words in that list are the
//!   constants of [`lang`].
//!
//! What it does **not** refuse is a key nothing uses; the constant is simply unused,
//! and `dead_code` does not fire inside a macro expansion.
//!
//! # What one language costs
//!
//! Nothing. With a single language declared, [`get`](Text::get) returns the one field
//! without consulting [`Language::current`], the call to `User::Language()` is never
//! emitted, and the image is byte-identical to the same program with the strings
//! written as plain `const`s.
pub use symbian_core::locale::{Language, lang};

/// Declares this module's translations. See [the module documentation](self).
#[macro_export]
macro_rules! locale {
    (
        languages: $default:ident $(, $other:ident)* $(,)? ;
        $( $key:ident = { $( $column:ident : $text:literal ),* $(,)? } ),* $(,)?
    ) => {
        /// One key's string in every language this module declares — the type the
        /// constants below have, and the reason a row that forgets a language is a
        /// compile error rather than a missing translation.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct Text {
            #[doc = concat!("`", stringify!($default), "`, the default.")]
            pub $default: &'static str,
            $(
                #[doc = concat!("`", stringify!($other), "`.")]
                pub $other: &'static str,
            )*
        }

        impl Text {
            /// The string for the language the device reports.
            ///
            /// The language is read from `User::Language()` **once** per process and
            /// cached; see [`Language::current`]($crate::locale::Language::current).
            pub fn get(self) -> &'static str {
                // A module with one language never asks the device anything: this is
                // a constant, the branch folds, and the euser import disappears with
                // it.
                const TRANSLATED: &[$crate::locale::Language] =
                    &[$($crate::locale::lang::$other),*];
                if TRANSLATED.is_empty() {
                    return self.$default;
                }
                self.get_in($crate::locale::Language::current())
            }

            /// The string a device set to `language` would get, without asking the
            /// device. The fallback chain is written out in
            /// [`symbian_core::locale`]($crate::locale).
            pub const fn get_in(self, language: $crate::locale::Language) -> &'static str {
                // 1. The language itself.
                let code = language.code();
                $(
                    if code == $crate::locale::lang::$other.code() {
                        return self.$other;
                    }
                )*
                // 2. The language a dialect is a dialect of.
                if let Some(base) = language.base() {
                    let code = base.code();
                    $(
                        if code == $crate::locale::lang::$other.code() {
                            return self.$other;
                        }
                    )*
                }
                // 3. The default.
                self.$default
            }
        }

        $(
            // A key is a constant, so its name should be upper case; it is allowed to
            // be lower case here because the declaration reads as a table and an
            // author may want it to look like one.
            #[allow(non_upper_case_globals)]
            pub const $key: Text = Text { $( $column: $text ),* };
        )*
    };
}
