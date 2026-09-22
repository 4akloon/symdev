//! The heap-accounting euser statics a test counts with.

unsafe extern "C" {
    /// `00000a5c T _ZN4User9AllocSizeERi` — `static TInt User::AllocSize(TInt&
    /// aTotalAllocSize)` (`e32std.h:4484`): the number of cells allocated on the current
    /// thread's heap, and their total size written through the argument. Non-leaving.
    /// It is the count the C++ parity baseline took (`docs/research/cpp-parity/`), so
    /// the two sides are measured the same way.
    #[link_name = "_ZN4User9AllocSizeERi"]
    pub fn User_AllocSize(total: *mut i32) -> i32;
}
