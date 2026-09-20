//! What an application crate needs beside its own `fn main()`: the process entry point
//! `eexe.lib` calls and the panic handler.
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

pub use exit_code::{ExitCode, IntoExitCode};
pub use {symbian_alloc, symbian_core, symbian_sys};

/// The process heap: the calling thread's Symbian heap.
#[global_allocator]
static HEAP: symbian_alloc::SymbianHeap = symbian_alloc::SymbianHeap;

/// An infallible allocation that failed ends the process with `User::Exit(KErrNoMemory)`
/// rather than becoming a Rust panic: `-4` is what a Symbian program reports for
/// out of memory, and nothing unwinds on the way out (design spec §3, §6). Code that
/// wants to survive a failed allocation uses the fallible `alloc` APIs (`try_reserve`,
/// `Vec::try_*`), which still see the null `User::Alloc` returned.
#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    symbian_alloc::oom()
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
#[macro_export]
macro_rules! entry {
    ($main:path) => {
        #[unsafe(export_name = "_Z7E32Mainv")]
        pub extern "C" fn __symbian_e32main() -> i32 {
            $crate::ExitCode::from_main($main())
        }
    };
}

/// A Rust panic ends the process with `User::Exit(-1)`: `panic = "abort"` means no
/// unwinding, and no Rust frame may ever be left for a C++ exception to cross (design
/// spec §3). A `User::Panic` with a category is the intended end state; the reason
/// mapping is not decided yet.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: `User::Exit` is a euser static member function (plain EABI, no `this`, no
    // ownership taken) that never returns; it is callable from any thread and takes the
    // process down without unwinding.
    unsafe { symbian_sys::euser::User_Exit(-1) }
}
