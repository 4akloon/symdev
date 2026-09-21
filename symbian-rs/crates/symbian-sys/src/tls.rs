//! Symbian's thread-local storage, from `nm -D
//! epoc32/release/armv5/lib/euser.dso`.
//!
//! # There is no `Dll` class in this SDK
//!
//! Every account of Symbian TLS names `Dll::Tls()`, `Dll::SetTls()` and
//! `Dll::FreeTls()` in `e32std.h`. On S60 3rd FP2 they do not exist:
//! `grep -rn 'class Dll\b\|Dll::Tls' epoc32/include/` matches one **comment**
//! (`banamedplugins.h:249`) and nothing else, `e32std.h` does not contain the string
//! `Tls` at all, and `nm -D` finds no `_ZN3Dll...` export in `euser.dso` or in any
//! other import library of this SDK.
//!
//! What does exist is `class UserSvr` in `e32svr.h` lines 39-43, five statics marked
//! `@internalAll` / `@internalComponent`. `Dll::Tls` is the thin inline wrapper a later
//! Symbian put over them; here the wrapper is missing and the calls are not, so the SDK
//! calls them directly. They are statics with scalar arguments, so experiment 78's
//! member ABI does not even come into it and no C++ shim is needed.
//!
//! # The handle is ours to choose
//!
//! `aHandle` is what `Dll::Tls` would have filled in with the calling DLL's code
//! segment handle. An EXE has no such handle to offer, so this crate passes a constant
//! of its own — see [`crate::tls::SYMBIAN_STD_TLS_HANDLE`] — and what that buys is
//! measured in experiment 88 rather than assumed.

use core::ffi::c_void;

/// The handle `symbian-std` uses for its one slot.
///
/// The value is arbitrary and only has to be stable and unlikely to collide with a
/// real DLL's code segment handle, which is a kernel handle and therefore small and
/// even. Measured in experiment 88: with this handle a set is read back, two handles do
/// not collide, and a second thread sees nothing the first stored.
pub const SYMBIAN_STD_TLS_HANDLE: i32 = 0x7359_6D64; // "sYmd"

unsafe extern "C" {
    /// `00001350 T _ZN7UserSvr9DllSetTlsEiPv` — `UserSvr::DllSetTls(TInt aHandle,
    /// TAny* aPtr)`, `e32svr.h` line 39. Returns a system error code
    /// (`KErrNoMemory` if the kernel cannot grow this thread's slot table).
    #[link_name = "_ZN7UserSvr9DllSetTlsEiPv"]
    pub fn UserSvr_DllSetTls(handle: i32, ptr: *mut c_void) -> i32;

    /// `00002200 T _ZN7UserSvr9DllSetTlsEiiPv` — `UserSvr::DllSetTls(TInt aHandle,
    /// TInt aDllUid, TAny* aPtr)`, `e32svr.h` line 40.
    #[link_name = "_ZN7UserSvr9DllSetTlsEiiPv"]
    pub fn UserSvr_DllSetTlsWithUid(handle: i32, dll_uid: i32, ptr: *mut c_void) -> i32;

    /// `0000133c T _ZN7UserSvr6DllTlsEi` — `UserSvr::DllTls(TInt aHandle)`,
    /// `e32svr.h` line 41. Null when this thread has stored nothing under `handle`.
    #[link_name = "_ZN7UserSvr6DllTlsEi"]
    pub fn UserSvr_DllTls(handle: i32) -> *mut c_void;

    /// `000021fc T _ZN7UserSvr6DllTlsEii` — `UserSvr::DllTls(TInt aHandle, TInt
    /// aDllUid)`, `e32svr.h` line 42.
    #[link_name = "_ZN7UserSvr6DllTlsEii"]
    pub fn UserSvr_DllTlsWithUid(handle: i32, dll_uid: i32) -> *mut c_void;

    /// `000012e4 T _ZN7UserSvr10DllFreeTlsEi` — `UserSvr::DllFreeTls(TInt aHandle)`,
    /// `e32svr.h` line 43: forgets this thread's slot. It frees the *slot*, never what
    /// the pointer in it refers to.
    #[link_name = "_ZN7UserSvr10DllFreeTlsEi"]
    pub fn UserSvr_DllFreeTls(handle: i32);
}
