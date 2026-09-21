//! Every string this example shows, in every language it speaks.
//!
//! The table is key-major on purpose: each row holds one key in all three languages,
//! so a row that forgets one does not compile. `english` is first, so it is the
//! default the fallback chain ends at.
//!
//! The three languages are chosen so the emulator can actually demonstrate the
//! difference. EKA2L1 will only let the device be set to a language the ROM's
//! `Z:\resource\bootdata\languages.txt` lists, and for RM-469 that is English,
//! French, German, Turkish, Dutch and Italian — so `french` is here to be switched
//! *to*, `ukrainian` is here as a language the table has and this device cannot
//! reach, and German is deliberately absent so that switching to it demonstrates the
//! fall back to the default.
symbian_std::locale! {
    languages: english, ukrainian, french;

    GREETING = {
        english:   "Hello from Rust",
        ukrainian: "Привіт з Rust",
        french:    "Bonjour depuis Rust",
    },
    LANGUAGE_IS = {
        english:   "language",
        ukrainian: "мова",
        french:    "langue",
    },
    OK = {
        english:   "ok",
        ukrainian: "гаразд",
        french:    "d'accord",
    },
}
