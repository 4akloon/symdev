//! Platform-specific extensions to `std` for Symbian OS 9.3 (S60 3rd FP2).
//!
//! There is exactly one thing here, and it is the one thing a Symbian application
//! cannot do without.

#![stable(feature = "symbian_std", since = "1.0.0")]
#![doc(cfg(target_os = "symbian"))]

use crate::process::Termination;

/// Runs `main` as the process's Rust entry point and returns the `TInt` `E32Main()`
/// gives the loader.
///
/// # Why an application has to call this
///
/// On every other platform the C runtime calls `main`, rustc generates the call to the
/// `start` lang item, and the linker ties the two together. Neither half of that
/// happens here:
///
/// - The image's entry point is the C++-mangled `E32Main()` that `eexe.lib`'s startup
///   reaches, not `main`.
/// - The application crate is compiled as a `staticlib`, because rustc never links on
///   this target — `symdev` owns the link line, which is byte-verified against the
///   SDK's own. rustc does not look for a `fn main` in a `staticlib`, so it generates
///   no call to the `start` lang item for anything to tie.
///
/// So the entry point is written by hand, by `#[symbian_std::main]`, and this is what
/// it calls. It does everything `lang_start` does — runtime initialisation, running
/// `main`, converting its [`Termination`] to an exit code, flushing `stdout` — and one
/// thing more: it runs the **main thread's** thread-local destructors on the way out.
/// Nothing in this kernel runs a destructor for any thread, and unlike a spawned
/// thread the main one has no trampoline to do it.
///
/// ```ignore
/// #[symbian_std::main]
/// fn main() -> std::io::Result<()> {
///     std::fs::write("E:\\hello.txt", b"hello")?;
///     Ok(())
/// }
/// ```
#[stable(feature = "symbian_std", since = "1.0.0")]
pub fn start<T: Termination + 'static>(main: fn() -> T) -> i32 {
    crate::rt::symbian_start(main)
}
