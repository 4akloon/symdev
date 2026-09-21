//! The device's UI language: `TLanguage`, and reading it once.

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
    /// **Not cached, and that is a measurement, not an oversight.** The obvious shape
    /// — read euser once into a `static` and load it afterwards — was built and
    /// measured, and on this CPU it is worse in both directions. ARMv5TE has no
    /// atomic instruction, so `AtomicU32::load(Relaxed)` compiles to
    /// `bl __atomic_load_4`, and that libcall is `symbian-libcalls`' lock-based
    /// emulation: an `RFastLock::Wait`/`Signal` pair, a kernel round trip, to avoid
    /// one call into euser. Linking it costs **794 bytes** of `.exe` in a `hello`-sized
    /// application (4 024 against 3 230 bytes, experiment 96) — a quarter of the whole
    /// image — for a read that is then *slower* than the one it replaced: the same
    /// 100 000-iteration loop takes 16 nanokernel ticks through the cache and **12**
    /// straight to euser. There is no `OnceCell` in `core` that avoids this, no
    /// `std::sync` on this path, and `has-thread-local` is false for the target.
    ///
    /// So the read is one euser call, measured at **0.12 µs** inside EKA2L1
    /// (100 000 calls in 12 nanokernel ticks of 1 000 µs, experiment 96; that is the
    /// emulator's dynarmic JIT, not an E52). An application that
    /// wants the language read exactly once holds it: `Language` is two bytes and
    /// `Copy`, so reading it into a local or a field and passing it to
    /// `Text::get_in` is the cache, and it costs nothing.
    ///
    /// A value outside `0..=0xFFFF` is not a `TLanguage` and becomes [`lang::none`];
    /// `User::Language()` has no error return, so this is a belt-and-braces narrowing
    /// rather than an observed case. `TODO: whether a language change under a running
    /// process is visible to it (not observed)` — nothing here assumes either way,
    /// because nothing here remembers.
    pub fn current() -> Self {
        // SAFETY: a euser static member function (plain EABI, no `this`) that takes
        // nothing, returns the enum in r0, touches no memory of ours and cannot leave.
        let raw = unsafe { User_Language() };
        match u16::try_from(raw) {
            Ok(code) => Self(code),
            Err(_) => lang::none,
        }
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
