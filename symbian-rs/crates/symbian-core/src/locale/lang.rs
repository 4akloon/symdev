//! Every `TLanguage` the SDK names, under the name an application writes in
//! [`locale!`](crate::locale!).
//!
//! Read off `epoc32/include/e32const.h` lines 1439-1784 (S60 3rd FP2): the
//! enumerator's `ELang` prefix dropped, the CamelCase broken at each capital, the
//! whole thing lower case. `ELangEnglish` is [`english`], `ELangSouthAfricanEnglish`
//! is [`south_african_english`], `ELangEnglish_Apac` is [`english_apac`]. Reading that
//! header needs `LC_ALL=C grep -a`: it is not ISO text.
//!
//! **Lower case on purpose.** These are the words a developer writes inside
//! `locale!`, where the same words become struct field names, and
//! `locale! { languages: english, ukrainian; ... }` is what a declaration should read
//! like. Rust's own name for a constant is upper case, so the lint is turned off here
//! and only here.
#![allow(non_upper_case_globals)]

use super::Language;

/// One line per enumerator: the name an application writes, its `TLanguage` value and
/// the `e32const.h` spelling the documentation quotes back.
macro_rules! languages {
    ($($name:ident = $code:literal ($sdk:literal),)*) => {
        $(
            #[doc = concat!("`", $sdk, " = ", stringify!($code), "`.")]
            pub const $name: Language = Language::from_code($code);
        )*
    };
}

languages! {
    test = 0 ("ELangTest"),
    english = 1 ("ELangEnglish"),
    french = 2 ("ELangFrench"),
    german = 3 ("ELangGerman"),
    spanish = 4 ("ELangSpanish"),
    italian = 5 ("ELangItalian"),
    swedish = 6 ("ELangSwedish"),
    danish = 7 ("ELangDanish"),
    norwegian = 8 ("ELangNorwegian"),
    finnish = 9 ("ELangFinnish"),
    american = 10 ("ELangAmerican"),
    swiss_french = 11 ("ELangSwissFrench"),
    swiss_german = 12 ("ELangSwissGerman"),
    portuguese = 13 ("ELangPortuguese"),
    turkish = 14 ("ELangTurkish"),
    icelandic = 15 ("ELangIcelandic"),
    russian = 16 ("ELangRussian"),
    hungarian = 17 ("ELangHungarian"),
    dutch = 18 ("ELangDutch"),
    belgian_flemish = 19 ("ELangBelgianFlemish"),
    australian = 20 ("ELangAustralian"),
    belgian_french = 21 ("ELangBelgianFrench"),
    austrian = 22 ("ELangAustrian"),
    new_zealand = 23 ("ELangNewZealand"),
    international_french = 24 ("ELangInternationalFrench"),
    czech = 25 ("ELangCzech"),
    slovak = 26 ("ELangSlovak"),
    polish = 27 ("ELangPolish"),
    slovenian = 28 ("ELangSlovenian"),
    taiwan_chinese = 29 ("ELangTaiwanChinese"),
    hong_kong_chinese = 30 ("ELangHongKongChinese"),
    prc_chinese = 31 ("ELangPrcChinese"),
    japanese = 32 ("ELangJapanese"),
    thai = 33 ("ELangThai"),
    afrikaans = 34 ("ELangAfrikaans"),
    albanian = 35 ("ELangAlbanian"),
    amharic = 36 ("ELangAmharic"),
    arabic = 37 ("ELangArabic"),
    armenian = 38 ("ELangArmenian"),
    tagalog = 39 ("ELangTagalog"),
    belarussian = 40 ("ELangBelarussian"),
    bengali = 41 ("ELangBengali"),
    bulgarian = 42 ("ELangBulgarian"),
    burmese = 43 ("ELangBurmese"),
    catalan = 44 ("ELangCatalan"),
    croatian = 45 ("ELangCroatian"),
    canadian_english = 46 ("ELangCanadianEnglish"),
    international_english = 47 ("ELangInternationalEnglish"),
    south_african_english = 48 ("ELangSouthAfricanEnglish"),
    estonian = 49 ("ELangEstonian"),
    farsi = 50 ("ELangFarsi"),
    canadian_french = 51 ("ELangCanadianFrench"),
    scots_gaelic = 52 ("ELangScotsGaelic"),
    georgian = 53 ("ELangGeorgian"),
    greek = 54 ("ELangGreek"),
    cyprus_greek = 55 ("ELangCyprusGreek"),
    gujarati = 56 ("ELangGujarati"),
    hebrew = 57 ("ELangHebrew"),
    hindi = 58 ("ELangHindi"),
    indonesian = 59 ("ELangIndonesian"),
    irish = 60 ("ELangIrish"),
    swiss_italian = 61 ("ELangSwissItalian"),
    kannada = 62 ("ELangKannada"),
    kazakh = 63 ("ELangKazakh"),
    khmer = 64 ("ELangKhmer"),
    korean = 65 ("ELangKorean"),
    lao = 66 ("ELangLao"),
    latvian = 67 ("ELangLatvian"),
    lithuanian = 68 ("ELangLithuanian"),
    macedonian = 69 ("ELangMacedonian"),
    malay = 70 ("ELangMalay"),
    malayalam = 71 ("ELangMalayalam"),
    marathi = 72 ("ELangMarathi"),
    moldavian = 73 ("ELangMoldavian"),
    mongolian = 74 ("ELangMongolian"),
    norwegian_nynorsk = 75 ("ELangNorwegianNynorsk"),
    brazilian_portuguese = 76 ("ELangBrazilianPortuguese"),
    punjabi = 77 ("ELangPunjabi"),
    romanian = 78 ("ELangRomanian"),
    serbian = 79 ("ELangSerbian"),
    sinhalese = 80 ("ELangSinhalese"),
    somali = 81 ("ELangSomali"),
    international_spanish = 82 ("ELangInternationalSpanish"),
    latin_american_spanish = 83 ("ELangLatinAmericanSpanish"),
    swahili = 84 ("ELangSwahili"),
    finland_swedish = 85 ("ELangFinlandSwedish"),
    reserved1 = 86 ("ELangReserved1"),
    tamil = 87 ("ELangTamil"),
    telugu = 88 ("ELangTelugu"),
    tibetan = 89 ("ELangTibetan"),
    tigrinya = 90 ("ELangTigrinya"),
    cyprus_turkish = 91 ("ELangCyprusTurkish"),
    turkmen = 92 ("ELangTurkmen"),
    ukrainian = 93 ("ELangUkrainian"),
    urdu = 94 ("ELangUrdu"),
    reserved2 = 95 ("ELangReserved2"),
    vietnamese = 96 ("ELangVietnamese"),
    welsh = 97 ("ELangWelsh"),
    zulu = 98 ("ELangZulu"),
    other = 99 ("ELangOther"),
    manufacturer_english = 100 ("ELangManufacturerEnglish"),
    south_sotho = 101 ("ELangSouthSotho"),
    english_apac = 129 ("ELangEnglish_Apac"),
    english_taiwan = 157 ("ELangEnglish_Taiwan"),
    english_hong_kong = 158 ("ELangEnglish_HongKong"),
    english_prc = 159 ("ELangEnglish_Prc"),
    english_japan = 160 ("ELangEnglish_Japan"),
    english_thailand = 161 ("ELangEnglish_Thailand"),
    malay_apac = 326 ("ELangMalay_Apac"),
    none = 0xFFFF ("ELangNone"),
}
