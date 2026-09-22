//! The device's UI language: `TLanguage`.

use symbian_sys::euser::User_Language;

/// One value of `TLanguage` (`e32const.h` line 1439).
///
/// A newtype over `u16` rather than an `enum` because the set is **open**: this SDK
/// names 108 of the 1024 the header's own comment allows, and a device may report one
/// it does not name. `u16` because `TLocale` stores these as `TUint16
/// iLanguageDowngrade[3]` (`e32std.h` line 2314) and `ELangNone = 0xFFFF` is the largest
/// enumerator.
///
/// A program does not need this to be localised — `BaflUtils::NearestLanguageFile`
/// chooses the strings file (see [`super::Str`]). It is here for the program that
/// wants to know the device's language itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Language(u16);

/// `ELangNone` (`e32const.h`): what an out-of-range answer is narrowed to.
const NONE: u16 = 0xFFFF;

impl Language {
    /// The language for a raw `TLanguage` value.
    pub const fn from_code(code: u16) -> Self {
        Self(code)
    }

    /// The raw `TLanguage` value.
    pub const fn code(self) -> u16 {
        self.0
    }

    /// What the device is set to (`User::Language()`), asked each time: one euser call,
    /// 0.12 µs in EKA2L1, and on ARMv5TE a cache is both bigger and slower than the call
    /// (experiment 96). A value outside `0..=0xFFFF` is not a `TLanguage` and becomes
    /// `ELangNone`; `User::Language()` has no error return, so that narrowing is
    /// belt-and-braces, not an observed case.
    pub fn current() -> Self {
        // SAFETY: a euser static member function (plain EABI, no `this`) that takes
        // nothing, returns the enum in r0, touches no memory of ours and cannot leave.
        let raw = unsafe { User_Language() };
        Self(u16::try_from(raw).unwrap_or(NONE))
    }
}
