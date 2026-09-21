//! Every string this example shows, in every language it speaks.
//!
//! The table is key-major on purpose: each row holds one key in all three languages,
//! so a row that forgets one does not compile. `english` is first, so it is the
//! default the fallback chain ends at.
symbian_std::locale! {
    languages: english, ukrainian, russian;

    GREETING = {
        english:   "Hello from Rust",
        ukrainian: "Привіт з Rust",
        russian:   "Привет из Rust",
    },
    LANGUAGE_IS = {
        english:   "language",
        ukrainian: "мова",
        russian:   "язык",
    },
    OK = {
        english:   "ok",
        ukrainian: "гаразд",
        russian:   "хорошо",
    },
}
