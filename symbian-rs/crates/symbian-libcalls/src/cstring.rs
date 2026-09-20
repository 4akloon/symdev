//! `memcmp` and `bcmp`: the two C runtime routines LLVM emits calls to and this
//! platform does not export.
//!
//! `euser.dso` exports `memcpy`, `memset`, `memmove` and `memclr`, and `drtaeabi.dso`
//! the whole `__aeabi_mem*` family, so those resolve from ROM — experiment 77 put both
//! DSOs before the Rust archive precisely so they would. **Nothing on the link line
//! defines `memcmp` or `bcmp`**: `nm -D` over euser, drtaeabi, dfpaeabi, scppnwdl and
//! drtrvct2_2 finds neither, and LLVM emits a call to one of them for `a == b` on two
//! `[u8]`. Comparing two byte slices in Rust did not link at all until these existed
//! (experiment 79).
//!
//! The alternative was `-Zbuild-std-features=compiler-builtins-mem`, which does define
//! them. Measured, that costs 4.6 kB: `compiler_builtins` is one codegen unit, so the
//! single `memcmp` reference also drags in `__adddf3`, `__divdf3`, `__muldf3` and the
//! single-precision trio, and `--gc-sections` cannot drop them because they are weak
//! *global* symbols and every dynamic symbol is a collection root in a `-shared` link.

/// `memcmp(const void*, const void*, size_t) -> int`.
///
/// ISO C 7.24.4.1: a byte-wise unsigned comparison of exactly `count` bytes. No Symbian
/// behaviour is being guessed at — the platform simply does not have this routine.
///
/// The pointer type is `*const c_void`, which is what rustc says this target's runtime
/// `memcmp` is — the `suspicious_runtime_symbol_definitions` lint prints the expected
/// signature, so the ABI here is taken from the compiler rather than assumed.
///
/// The loop reads through [`read_volatile`](core::ptr::read_volatile), which is not
/// decoration: LLVM recognises a plain byte-comparison loop and rewrites it into a call
/// to `memcmp` or `bcmp`, and this function *is* `memcmp`.
///
/// # Safety
/// `left` and `right` are readable for `count` bytes. The caller is either compiler-
/// generated code or C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(
    left: *const core::ffi::c_void,
    right: *const core::ffi::c_void,
    count: usize,
) -> i32 {
    let (left, right) = (left.cast::<u8>(), right.cast::<u8>());
    let mut i = 0;
    while i < count {
        // SAFETY: `i < count` and both pointers are readable for `count` bytes.
        let (l, r) = unsafe {
            (
                core::ptr::read_volatile(left.add(i)),
                core::ptr::read_volatile(right.add(i)),
            )
        };
        if l != r {
            return l as i32 - r as i32;
        }
        i += 1;
    }
    0
}

/// `bcmp(const void*, const void*, size_t) -> int`.
///
/// LLVM emits this instead of `memcmp` when only equality matters, so a build that has
/// one and not the other still fails to link. The same comparison, with a weaker
/// contract: any non-zero value means "different".
///
/// # Safety
/// As [`memcmp`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(
    left: *const core::ffi::c_void,
    right: *const core::ffi::c_void,
    count: usize,
) -> i32 {
    // SAFETY: the caller's contract is `memcmp`'s contract.
    unsafe { memcmp(left, right, count) }
}
