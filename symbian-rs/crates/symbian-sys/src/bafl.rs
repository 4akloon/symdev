//! `bafl.dso`: the one export the localised-strings reader calls directly.
use crate::des16::TDes16;
use crate::efsrv::RFs;

unsafe extern "C" {
    /// `0000044c T _ZN9BaflUtils19NearestLanguageFileERK3RFsR4TBufILi256EE` —
    /// `static void BaflUtils::NearestLanguageFile(const RFs& aFs, TFileName& aName)`
    /// (`bautils.h` line 61): rewrites `aName` in place to the variant of the file that
    /// exists for the device language (`.r02`, …), or leaves it as it was.
    ///
    /// **It does not leave**, and that was followed rather than read off the name: its
    /// call closure in the emulator ROM's `bafl.dll` reaches only descriptor, `TParse`,
    /// `RFs::Entry`, `RDir`, `User::Language`, `TLocale`, `HAL::Get`, `UserSvr::DllTls`
    /// and runtime helpers — no `User::Leave*` and no `L` function (experiment 102).
    ///
    /// `name` must point at a real `TBuf16<256>`: the argument is a `TFileName&`, and the
    /// mangled name says so.
    #[link_name = "_ZN9BaflUtils19NearestLanguageFileERK3RFsR4TBufILi256EE"]
    pub fn BaflUtils_NearestLanguageFile(fs: *const RFs, name: *mut TDes16);
}
