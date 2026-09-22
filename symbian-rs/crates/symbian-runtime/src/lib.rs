//! What an application crate needs beside its own `fn main()`: the process entry point
//! `eexe.lib` calls, the panic handler and the out-of-memory handler (`panic.rs`).
//!
//! `eexe.lib`'s `_E32Startup` reaches the C++-mangled `E32Main()` (`_Z7E32Mainv`,
//! returning `TInt`); symdev's link line names it with `-u _Z7E32Mainv` so the archive
//! member that defines it is pulled (experiment 65a). That definition has to be in the
//! application crate, where it can name the crate's own `main`: `#[symbian_std::main]`
//! writes it from an attribute and [`entry!`] from a macro, and both end in
//! [`ExitCode::from_main`].
//!
//! The crate also installs the one heap (experiment 68): `#[global_allocator]` may be
//! written once per program, so putting it here means an application cannot forget it
//! and two libraries cannot each install one.
#![no_std]
#![feature(alloc_error_handler)]

mod exit_code;
mod panic;

pub use exit_code::{ExitCode, IntoExitCode};
pub use {symbian_alloc, symbian_core, symbian_sys};

/// The process heap: the calling thread's Symbian heap.
#[global_allocator]
static HEAP: symbian_alloc::SymbianHeap = symbian_alloc::SymbianHeap;

/// The body of a `no_std` `E32Main`: install the thread's cleanup stack, run `main`,
/// convert what it returned, free the cleanup stack.
///
/// Both entry points call this so that there is one answer to "what does starting a
/// Rust program on this platform do": [`entry!`] for a crate that names its own
/// function, and `#[symbian_std::main]` through `symbian_std::__start`. The `std`
/// shape's own `__start` reaches `std::os::symbian::start`, which installs the same
/// [`symbian_sys::cleanup::TrapCleanup`] — that equality is the point, and
/// `examples/cleanup` is the test that holds it.
pub fn start<T: IntoExitCode>(main: fn() -> T) -> i32 {
    // Held for the whole of `main` and dropped after it: the destructor uninstalls
    // the handler and frees the stack, so it must outlive every frame that could push
    // onto it.
    let _cleanup = symbian_sys::cleanup::TrapCleanup::install();
    ExitCode::from_main(main())
}

/// Declares a function as the application's entry point, by name.
///
/// ```ignore
/// #![no_std]
/// fn main() { /* ... */ }
/// symbian_runtime::entry!(main);
/// ```
///
/// `#[symbian_std::main]` is the shape an application should use (design spec §6a);
/// this macro stays for a crate that wants to name its own function, and for the
/// layer below `symbian-std`. Both write the same wrapper and both convert through
/// [`IntoExitCode`].
///
/// # The cleanup stack
///
/// The wrapper opens with `CTrapCleanup::New()`, exactly as a C++ `E32Main`
/// conventionally does, and frees it after `main` returns. Without it the first
/// `CleanupStack::PushL` anywhere below — including inside an SDK call's own `TRAP`,
/// where the application never sees the call — panics `E32USER-CBase 69` and takes
/// the thread with it, uncatchably. That is measured: `RFs::GetDir` from a `no_std`
/// console application died with exactly that line until this was here, while the
/// same call under `std` worked, because `std` had been installing one all along.
/// See [`symbian_sys::cleanup`].
///
/// An Avkon application does not come through here: `#[symbian_std::main(gui)]`
/// writes no `E32Main` at all, because the C++ shim owns the entry point and hands
/// the process to `EikStart::RunApplication`, which installs the thread's cleanup
/// stack itself.
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(export_name = "_Z7E32Mainv")]
        pub extern "C" fn __symbian_e32main() -> i32 {
            $crate::start($main)
        }
    };
}
