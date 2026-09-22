//! How a `no_std` program ends when it cannot go on: a Rust panic and an allocation that
//! cannot be satisfied both become a Symbian panic with the category `RUST`, told apart
//! by the reason (`KErrGeneral` and `KErrNoMemory`). Nothing unwinds on either path
//! (`panic = "abort"`, design spec §3): no Rust frame may ever be left for a C++
//! exception to cross.

use symbian_sys::des::Lit16;

/// The panic category, `RUST`: the name the emulator log gives the death
/// (`panicked with category: RUST`), where a C++ program's own panics show theirs.
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
/// (experiment 100 measures both).
const KERR_GENERAL: i32 = -2;

/// A Rust panic ends the process with `User::Panic("RUST", KErrGeneral)`, the way a C++
/// program on this platform dies on purpose (observed: `Thread Main panicked with
/// category: RUST and exit code: -2`, experiment 100).
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: `User::Panic` is a euser static member function (plain EABI, no `this`)
    // that never returns and is callable from any thread. The category is a `'static`
    // literal with the observed `_LIT16` layout; euser copies it before the thread dies.
    unsafe { symbian_sys::euser::User_Panic(CATEGORY.as_desc(), KERR_GENERAL) }
}

/// An infallible allocation that failed ends the process with
/// `User::Panic("RUST", KErrNoMemory)`: the same category as a Rust panic, and `-4`, the
/// code a Symbian program reports for out of memory, as the reason that tells the two
/// apart. Code that wants to survive a failed allocation uses the fallible `alloc` APIs
/// (`try_reserve`, `Vec::try_*`), which still see the null `User::Alloc` returned.
///
/// Why a panic and not `User::Exit(KErrNoMemory)`, which is what this was before
/// experiment 100: the failure happens deep inside `main`, with the thread's
/// `CTrapCleanup` installed, and `User::Exit` in that state was observed to die `KERN-EXEC 3` in the emulator — a C++
/// `E32Main` calling `User::Exit` after `CTrapCleanup::New()` dies the same way — so the
/// `-4` never reached the log. A C++ program reports `-4` as an exit only when a top-level
/// `TRAPD` catches the leave and `E32Main` returns after deleting its cleanup stack, which
/// an abort that never unwinds cannot do; the same leave without a `TRAP` is itself a
/// panic (`E32USER-CBase 65`).
#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    // SAFETY: as in `panic` above.
    unsafe { symbian_sys::euser::User_Panic(CATEGORY.as_desc(), symbian_alloc::KERR_NO_MEMORY) }
}
