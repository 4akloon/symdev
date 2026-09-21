//! The device's UI language: `TLanguage`, and reading it once.

use core::sync::atomic::{AtomicU32, Ordering};

use symbian_sys::euser::User_Language;

use super::lang;

/// One value of `TLanguage` (`e32const.h` line 1439).
///
/// It is a newtype over `u16` rather than an `enum` because the set is **open**: this
/// SDK names 108 of the 1024 the header's own comment allows, and a device may report
/// one it does not name. [`lang`] holds every named value under the SDK's own spelling
/// with `ELang` dropped and the CamelCase broken at each capital — `ELangEnglish` is
/// [`lang::english`], `ELangEnglish_Apac` is [`lang::english_apac`].
///
/// `u16` and not `i32`: `TLocale` stores three of these as `TUint16
/// iLanguageDowngrade[3]` (`e32std.h` line 2314), and `ELangNone = 0xFFFF` is the
/// largest enumerator, so sixteen bits is the platform's own width for the type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Language(u16);

/// The cache behind [`Language::current`]. `u32::MAX` is "not read yet"; it cannot
/// collide with a language, which is a `u16`.
static CURRENT: AtomicU32 = AtomicU32::new(u32::MAX);

const UNREAD: u32 = u32::MAX;

impl Language {
    /// The language for a raw `TLanguage` value.
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    /// The raw `TLanguage` value.
    pub const fn code(self) -> u16 {
        self.0
    }

    /// What the device's UI is set to, from `User::Language()`.
    ///
    /// **Read once.** The first call asks euser and caches the answer; every later
    /// call is one relaxed load. There is no `std::sync` on this path and no
    /// `OnceCell` in `core`, so the cache is a plain [`AtomicU32`]: two threads racing
    /// here both call `User::Language()` and both store the same value, which is why
    /// `Relaxed` is enough and why no lock is needed. Nothing in this SDK observed the
    /// language changing under a running process, and the platform offers no
    /// notification for it that was observed — `TODO: whether a live language change
    /// is visible to a running process (not observed)`.
    ///
    /// A value outside `0..=0xFFFF` is not a `TLanguage` and becomes
    /// [`lang::none`]; `User::Language()` has no error return, so this is a
    /// belt-and-braces narrowing rather than an observed case.
    pub fn current() -> Self {
        match CURRENT.load(Ordering::Relaxed) {
            UNREAD => {
                // SAFETY: a euser static member function (plain EABI, no `this`) that
                // takes nothing, returns the enum in r0, touches no memory of ours and
                // cannot leave.
                let raw = unsafe { User_Language() };
                let language = match u16::try_from(raw) {
                    Ok(code) => Self(code),
                    Err(_) => lang::none,
                };
                CURRENT.store(u32::from(language.0), Ordering::Relaxed);
                language
            }
            cached => Self(cached as u16),
        }
    }

    /// Pretends the device reports `language`, for a test that cannot reach euser.
    ///
    /// It writes the same cache [`Language::current`] reads, so it must run before the
    /// first `current()` to have any effect. Present so an example can show the
    /// fallback chain choosing differently without a second device.
    pub fn set_current(language: Language) {
        CURRENT.store(u32::from(language.0), Ordering::Relaxed);
    }

    /// The language a dialect is a dialect **of**, or `None` for a language that is
    /// not one.
    ///
    /// This is the only parent/child relation `e32const.h` declares: seven of its
    /// enumerators carry an `_Suffix` naming a region over a language it already has
    /// (`ELangEnglish_Apac` over `ELangEnglish`, `ELangMalay_Apac` over `ELangMalay`),
    /// and the rest do not. `ELangAmerican`, `ELangCanadianEnglish`,
    /// `ELangInternationalEnglish`, `ELangSouthAfricanEnglish`, `ELangAustralian` and
    /// `ELangNewZealand` are English to a reader but the header says nothing that
    /// makes them English to a program, so this returns `None` for them and an
    /// application that wants them says so in its own `locale!` list.
    pub const fn base(self) -> Option<Language> {
        match self.0 {
            c if c == lang::english_apac.0
                || c == lang::english_taiwan.0
                || c == lang::english_hong_kong.0
                || c == lang::english_prc.0
                || c == lang::english_japan.0
                || c == lang::english_thailand.0 =>
            {
                Some(lang::english)
            }
            c if c == lang::malay_apac.0 => Some(lang::malay),
            _ => None,
        }
    }
}
/// Checked where every other target-side invariant in this workspace is: at compile
/// time, because a `#[test]` cannot run here — `symbian-rs` builds for
/// `arm-symbian-e32`, which has no test harness.
const _: () = {
    // The codes, against the header.
    assert!(lang::english.code() == 1);
    assert!(lang::russian.code() == 16);
    assert!(lang::polish.code() == 27);
    assert!(lang::ukrainian.code() == 93);
    assert!(lang::english_apac.code() == 129);
    assert!(lang::none.code() == 0xFFFF);
    assert!(Language::from_code(93).code() == lang::ukrainian.code());

    // Every `_Suffix` enumerator falls back to the language it is suffixed over.
    assert!(matches!(lang::english_apac.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::english_taiwan.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::english_hong_kong.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::english_prc.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::english_japan.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::english_thailand.base(), Some(l) if l.code() == 1));
    assert!(matches!(lang::malay_apac.base(), Some(l) if l.code() == 70));

    // And nothing else does: a language the header declares no parent for has none,
    // however English it reads.
    assert!(lang::english.base().is_none());
    assert!(lang::american.base().is_none());
    assert!(lang::canadian_english.base().is_none());
    assert!(lang::international_english.base().is_none());
    assert!(lang::ukrainian.base().is_none());
};
