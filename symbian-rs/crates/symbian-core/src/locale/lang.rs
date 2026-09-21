//! Every `TLanguage` the SDK names, under the name an application writes in
//! [`locale!`](crate::locale!).
//!
//! Read off `epoc32/include/e32const.h` lines 1439-1784 (S60 3rd FP2), one constant
//! per enumerator, in the header's order: the `ELang` prefix dropped, the CamelCase
//! broken at each capital, the whole thing lower case. `ELangEnglish` is [`english`],
//! `ELangSouthAfricanEnglish` is [`south_african_english`], `ELangEnglish_Apac` is
//! [`english_apac`]. Reading that header needs `LC_ALL=C grep -a`: it is not ISO text.
//!
//! **Lower case on purpose.** These are the words a developer writes inside
//! `locale!`, where the same words become struct field names, and
//! `locale! { languages: english, ukrainian; ... }` is what a declaration should read
//! like. Rust's own name for a constant is upper case, so the lint is turned off here
//! and only here. They are written out one by one rather than produced by a macro so
//! that rustc's "a constant with a similar name exists" points at the line a
//! developer wants to see.
#![allow(non_upper_case_globals)]

use super::Language;

pub const test: Language = Language::from_code(0); // ELangTest
pub const english: Language = Language::from_code(1); // ELangEnglish
pub const french: Language = Language::from_code(2); // ELangFrench
pub const german: Language = Language::from_code(3); // ELangGerman
pub const spanish: Language = Language::from_code(4); // ELangSpanish
pub const italian: Language = Language::from_code(5); // ELangItalian
pub const swedish: Language = Language::from_code(6); // ELangSwedish
pub const danish: Language = Language::from_code(7); // ELangDanish
pub const norwegian: Language = Language::from_code(8); // ELangNorwegian
pub const finnish: Language = Language::from_code(9); // ELangFinnish
pub const american: Language = Language::from_code(10); // ELangAmerican
pub const swiss_french: Language = Language::from_code(11); // ELangSwissFrench
pub const swiss_german: Language = Language::from_code(12); // ELangSwissGerman
pub const portuguese: Language = Language::from_code(13); // ELangPortuguese
pub const turkish: Language = Language::from_code(14); // ELangTurkish
pub const icelandic: Language = Language::from_code(15); // ELangIcelandic
pub const russian: Language = Language::from_code(16); // ELangRussian
pub const hungarian: Language = Language::from_code(17); // ELangHungarian
pub const dutch: Language = Language::from_code(18); // ELangDutch
pub const belgian_flemish: Language = Language::from_code(19); // ELangBelgianFlemish
pub const australian: Language = Language::from_code(20); // ELangAustralian
pub const belgian_french: Language = Language::from_code(21); // ELangBelgianFrench
pub const austrian: Language = Language::from_code(22); // ELangAustrian
pub const new_zealand: Language = Language::from_code(23); // ELangNewZealand
pub const international_french: Language = Language::from_code(24); // ELangInternationalFrench
pub const czech: Language = Language::from_code(25); // ELangCzech
pub const slovak: Language = Language::from_code(26); // ELangSlovak
pub const polish: Language = Language::from_code(27); // ELangPolish
pub const slovenian: Language = Language::from_code(28); // ELangSlovenian
pub const taiwan_chinese: Language = Language::from_code(29); // ELangTaiwanChinese
pub const hong_kong_chinese: Language = Language::from_code(30); // ELangHongKongChinese
pub const prc_chinese: Language = Language::from_code(31); // ELangPrcChinese
pub const japanese: Language = Language::from_code(32); // ELangJapanese
pub const thai: Language = Language::from_code(33); // ELangThai
pub const afrikaans: Language = Language::from_code(34); // ELangAfrikaans
pub const albanian: Language = Language::from_code(35); // ELangAlbanian
pub const amharic: Language = Language::from_code(36); // ELangAmharic
pub const arabic: Language = Language::from_code(37); // ELangArabic
pub const armenian: Language = Language::from_code(38); // ELangArmenian
pub const tagalog: Language = Language::from_code(39); // ELangTagalog
pub const belarussian: Language = Language::from_code(40); // ELangBelarussian
pub const bengali: Language = Language::from_code(41); // ELangBengali
pub const bulgarian: Language = Language::from_code(42); // ELangBulgarian
pub const burmese: Language = Language::from_code(43); // ELangBurmese
pub const catalan: Language = Language::from_code(44); // ELangCatalan
pub const croatian: Language = Language::from_code(45); // ELangCroatian
pub const canadian_english: Language = Language::from_code(46); // ELangCanadianEnglish
pub const international_english: Language = Language::from_code(47); // ELangInternationalEnglish
pub const south_african_english: Language = Language::from_code(48); // ELangSouthAfricanEnglish
pub const estonian: Language = Language::from_code(49); // ELangEstonian
pub const farsi: Language = Language::from_code(50); // ELangFarsi
pub const canadian_french: Language = Language::from_code(51); // ELangCanadianFrench
pub const scots_gaelic: Language = Language::from_code(52); // ELangScotsGaelic
pub const georgian: Language = Language::from_code(53); // ELangGeorgian
pub const greek: Language = Language::from_code(54); // ELangGreek
pub const cyprus_greek: Language = Language::from_code(55); // ELangCyprusGreek
pub const gujarati: Language = Language::from_code(56); // ELangGujarati
pub const hebrew: Language = Language::from_code(57); // ELangHebrew
pub const hindi: Language = Language::from_code(58); // ELangHindi
pub const indonesian: Language = Language::from_code(59); // ELangIndonesian
pub const irish: Language = Language::from_code(60); // ELangIrish
pub const swiss_italian: Language = Language::from_code(61); // ELangSwissItalian
pub const kannada: Language = Language::from_code(62); // ELangKannada
pub const kazakh: Language = Language::from_code(63); // ELangKazakh
pub const khmer: Language = Language::from_code(64); // ELangKhmer
pub const korean: Language = Language::from_code(65); // ELangKorean
pub const lao: Language = Language::from_code(66); // ELangLao
pub const latvian: Language = Language::from_code(67); // ELangLatvian
pub const lithuanian: Language = Language::from_code(68); // ELangLithuanian
pub const macedonian: Language = Language::from_code(69); // ELangMacedonian
pub const malay: Language = Language::from_code(70); // ELangMalay
pub const malayalam: Language = Language::from_code(71); // ELangMalayalam
pub const marathi: Language = Language::from_code(72); // ELangMarathi
pub const moldavian: Language = Language::from_code(73); // ELangMoldavian
pub const mongolian: Language = Language::from_code(74); // ELangMongolian
pub const norwegian_nynorsk: Language = Language::from_code(75); // ELangNorwegianNynorsk
pub const brazilian_portuguese: Language = Language::from_code(76); // ELangBrazilianPortuguese
pub const punjabi: Language = Language::from_code(77); // ELangPunjabi
pub const romanian: Language = Language::from_code(78); // ELangRomanian
pub const serbian: Language = Language::from_code(79); // ELangSerbian
pub const sinhalese: Language = Language::from_code(80); // ELangSinhalese
pub const somali: Language = Language::from_code(81); // ELangSomali
pub const international_spanish: Language = Language::from_code(82); // ELangInternationalSpanish
pub const latin_american_spanish: Language = Language::from_code(83); // ELangLatinAmericanSpanish
pub const swahili: Language = Language::from_code(84); // ELangSwahili
pub const finland_swedish: Language = Language::from_code(85); // ELangFinlandSwedish
pub const reserved1: Language = Language::from_code(86); // ELangReserved1
pub const tamil: Language = Language::from_code(87); // ELangTamil
pub const telugu: Language = Language::from_code(88); // ELangTelugu
pub const tibetan: Language = Language::from_code(89); // ELangTibetan
pub const tigrinya: Language = Language::from_code(90); // ELangTigrinya
pub const cyprus_turkish: Language = Language::from_code(91); // ELangCyprusTurkish
pub const turkmen: Language = Language::from_code(92); // ELangTurkmen
pub const ukrainian: Language = Language::from_code(93); // ELangUkrainian
pub const urdu: Language = Language::from_code(94); // ELangUrdu
pub const reserved2: Language = Language::from_code(95); // ELangReserved2
pub const vietnamese: Language = Language::from_code(96); // ELangVietnamese
pub const welsh: Language = Language::from_code(97); // ELangWelsh
pub const zulu: Language = Language::from_code(98); // ELangZulu
pub const other: Language = Language::from_code(99); // ELangOther
pub const manufacturer_english: Language = Language::from_code(100); // ELangManufacturerEnglish
pub const south_sotho: Language = Language::from_code(101); // ELangSouthSotho
pub const english_apac: Language = Language::from_code(129); // ELangEnglish_Apac
pub const english_taiwan: Language = Language::from_code(157); // ELangEnglish_Taiwan
pub const english_hong_kong: Language = Language::from_code(158); // ELangEnglish_HongKong
pub const english_prc: Language = Language::from_code(159); // ELangEnglish_Prc
pub const english_japan: Language = Language::from_code(160); // ELangEnglish_Japan
pub const english_thailand: Language = Language::from_code(161); // ELangEnglish_Thailand
pub const malay_apac: Language = Language::from_code(326); // ELangMalay_Apac
pub const none: Language = Language::from_code(0xFFFF); // ELangNone
