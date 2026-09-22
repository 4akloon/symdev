//! `Language`: a `TLanguage` by the name a locales file is called.
//!
//! Read off `epoc32/include/e32const.h` lines 1439-1784 (S60 3rd FP2), one row per
//! enumerator in the header's order: `ELang` dropped, CamelCase broken into lower-case
//! words — `ELangEnglish` is `english`, `ELangEnglish_Apac` is `english_apac`. Reading
//! that header needs `LC_ALL=C grep -a`: it is not ISO text.
//!
//! `ELangTest` (0) and `ELangNone` (0xFFFF) are left out: neither names a language a
//! device can be set to, and a file called after either would be a mistake.

/// One language: the file-name word and the `TLanguage` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub name: &'static str,
    pub code: u16,
}

impl Language {
    /// The language a `locales/<name>.toml` file is for.
    pub fn named(name: &str) -> Option<Language> {
        TABLE
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(name, code)| Language { name, code })
    }

    /// The resource-file extension the platform looks for: `r` and the value, at least
    /// two digits — `r01`, `r93`, `r129` — as `BaflUtils::NearestLanguageFile` expects.
    pub fn suffix(&self) -> String {
        format!("r{:02}", self.code)
    }
}

/// `(name, TLanguage)` for every language `e32const.h` declares.
const TABLE: &[(&str, u16)] = &[
    ("english", 1),                 // ELangEnglish
    ("french", 2),                  // ELangFrench
    ("german", 3),                  // ELangGerman
    ("spanish", 4),                 // ELangSpanish
    ("italian", 5),                 // ELangItalian
    ("swedish", 6),                 // ELangSwedish
    ("danish", 7),                  // ELangDanish
    ("norwegian", 8),               // ELangNorwegian
    ("finnish", 9),                 // ELangFinnish
    ("american", 10),               // ELangAmerican
    ("swiss_french", 11),           // ELangSwissFrench
    ("swiss_german", 12),           // ELangSwissGerman
    ("portuguese", 13),             // ELangPortuguese
    ("turkish", 14),                // ELangTurkish
    ("icelandic", 15),              // ELangIcelandic
    ("russian", 16),                // ELangRussian
    ("hungarian", 17),              // ELangHungarian
    ("dutch", 18),                  // ELangDutch
    ("belgian_flemish", 19),        // ELangBelgianFlemish
    ("australian", 20),             // ELangAustralian
    ("belgian_french", 21),         // ELangBelgianFrench
    ("austrian", 22),               // ELangAustrian
    ("new_zealand", 23),            // ELangNewZealand
    ("international_french", 24),   // ELangInternationalFrench
    ("czech", 25),                  // ELangCzech
    ("slovak", 26),                 // ELangSlovak
    ("polish", 27),                 // ELangPolish
    ("slovenian", 28),              // ELangSlovenian
    ("taiwan_chinese", 29),         // ELangTaiwanChinese
    ("hong_kong_chinese", 30),      // ELangHongKongChinese
    ("prc_chinese", 31),            // ELangPrcChinese
    ("japanese", 32),               // ELangJapanese
    ("thai", 33),                   // ELangThai
    ("afrikaans", 34),              // ELangAfrikaans
    ("albanian", 35),               // ELangAlbanian
    ("amharic", 36),                // ELangAmharic
    ("arabic", 37),                 // ELangArabic
    ("armenian", 38),               // ELangArmenian
    ("tagalog", 39),                // ELangTagalog
    ("belarussian", 40),            // ELangBelarussian
    ("bengali", 41),                // ELangBengali
    ("bulgarian", 42),              // ELangBulgarian
    ("burmese", 43),                // ELangBurmese
    ("catalan", 44),                // ELangCatalan
    ("croatian", 45),               // ELangCroatian
    ("canadian_english", 46),       // ELangCanadianEnglish
    ("international_english", 47),  // ELangInternationalEnglish
    ("south_african_english", 48),  // ELangSouthAfricanEnglish
    ("estonian", 49),               // ELangEstonian
    ("farsi", 50),                  // ELangFarsi
    ("canadian_french", 51),        // ELangCanadianFrench
    ("scots_gaelic", 52),           // ELangScotsGaelic
    ("georgian", 53),               // ELangGeorgian
    ("greek", 54),                  // ELangGreek
    ("cyprus_greek", 55),           // ELangCyprusGreek
    ("gujarati", 56),               // ELangGujarati
    ("hebrew", 57),                 // ELangHebrew
    ("hindi", 58),                  // ELangHindi
    ("indonesian", 59),             // ELangIndonesian
    ("irish", 60),                  // ELangIrish
    ("swiss_italian", 61),          // ELangSwissItalian
    ("kannada", 62),                // ELangKannada
    ("kazakh", 63),                 // ELangKazakh
    ("khmer", 64),                  // ELangKhmer
    ("korean", 65),                 // ELangKorean
    ("lao", 66),                    // ELangLao
    ("latvian", 67),                // ELangLatvian
    ("lithuanian", 68),             // ELangLithuanian
    ("macedonian", 69),             // ELangMacedonian
    ("malay", 70),                  // ELangMalay
    ("malayalam", 71),              // ELangMalayalam
    ("marathi", 72),                // ELangMarathi
    ("moldavian", 73),              // ELangMoldavian
    ("mongolian", 74),              // ELangMongolian
    ("norwegian_nynorsk", 75),      // ELangNorwegianNynorsk
    ("brazilian_portuguese", 76),   // ELangBrazilianPortuguese
    ("punjabi", 77),                // ELangPunjabi
    ("romanian", 78),               // ELangRomanian
    ("serbian", 79),                // ELangSerbian
    ("sinhalese", 80),              // ELangSinhalese
    ("somali", 81),                 // ELangSomali
    ("international_spanish", 82),  // ELangInternationalSpanish
    ("latin_american_spanish", 83), // ELangLatinAmericanSpanish
    ("swahili", 84),                // ELangSwahili
    ("finland_swedish", 85),        // ELangFinlandSwedish
    ("reserved1", 86),              // ELangReserved1
    ("tamil", 87),                  // ELangTamil
    ("telugu", 88),                 // ELangTelugu
    ("tibetan", 89),                // ELangTibetan
    ("tigrinya", 90),               // ELangTigrinya
    ("cyprus_turkish", 91),         // ELangCyprusTurkish
    ("turkmen", 92),                // ELangTurkmen
    ("ukrainian", 93),              // ELangUkrainian
    ("urdu", 94),                   // ELangUrdu
    ("reserved2", 95),              // ELangReserved2
    ("vietnamese", 96),             // ELangVietnamese
    ("welsh", 97),                  // ELangWelsh
    ("zulu", 98),                   // ELangZulu
    ("other", 99),                  // ELangOther
    ("manufacturer_english", 100),  // ELangManufacturerEnglish
    ("south_sotho", 101),           // ELangSouthSotho
    ("english_apac", 129),          // ELangEnglish_Apac
    ("english_taiwan", 157),        // ELangEnglish_Taiwan
    ("english_hong_kong", 158),     // ELangEnglish_HongKong
    ("english_prc", 159),           // ELangEnglish_Prc
    ("english_japan", 160),         // ELangEnglish_Japan
    ("english_thailand", 161),      // ELangEnglish_Thailand
    ("malay_apac", 326),            // ELangMalay_Apac
];
