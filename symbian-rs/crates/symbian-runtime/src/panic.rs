//! How a `no_std` program ends when it cannot go on: a Rust panic becomes a Symbian
//! panic with a named category, and an allocation that cannot be satisfied becomes an
//! exit with `KErrNoMemory`. Nothing unwinds on either path (`panic = "abort"`, design
//! spec §3): no Rust frame may ever be left for a C++ exception to cross.

use symbian_sys::des::Lit16;

/// The panic category, `RUST`: what the emulator log and a device's crash report name
/// as the *reason kind* of the process's death.
///
/// It is the category the `std` platform layer's `abort_internal` already raises
/// (`rust-src/overlay/library/std/src/sys/pal/symbian/mod.rs`), so a Rust panic reads
/// the same whichever library shape the program was built on. Four UTF-16 units, well
/// inside `KMaxExitCategoryName` (`0x10`, `e32const.h`), the capacity of the
/// `TExitCategoryName` a process's exit category is read back into (`e32cmn.h`).
static CATEGORY: Lit16<4> = Lit16::ascii(b"RUST");

/// `KErrGeneral` (`e32err.h`): the panic reason, the same one `std` uses. The panic's
/// message and location are not carried: building either costs `core::fmt` or the
/// `Location` statics of every panic site, which is the cost this path exists to avoid
/// (experiment 99 measures both).
const KERR_GENERAL: i32 = -2;

/// A Rust panic ends the process with `User::Panic("RUST", KErrGeneral)`, the way a C++
/// program on this platform dies on purpose.
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: `User::Panic` is a euser static member function (plain EABI, no `this`)
    // that never returns and is callable from any thread. The category is a `'static`
    // literal with the observed `_LIT16` layout; euser copies it before the thread dies.
    unsafe { symbian_sys::euser::User_Panic(CATEGORY.as_desc(), KERR_GENERAL) }
}

/// An infallible allocation that failed ends the process with `User::Exit(KErrNoMemory)`
/// rather than a panic: `-4` is what a Symbian program reports for out of memory, and
/// what a conventional C++ `E32Main` returns when its top-level `TRAPD` catches the
/// `KErrNoMemory` leave of a failed `new (ELeave)` (experiment 99). Code that wants to
/// survive a failed allocation uses the fallible `alloc` APIs (`try_reserve`,
/// `Vec::try_*`), which still see the null `User::Alloc` returned.
#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    symbian_alloc::oom()
}
