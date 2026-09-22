//! How a `no_std` program dies (experiment 99): run it and read the emulator's log.
//!
//! It fails on purpose, in one of two ways, so both ends of `symbian-runtime`'s
//! `panic.rs` can be seen from outside the process:
//!
//! - by default it indexes past the end of a slice, a Rust panic, and the process ends
//!   with `User::Panic("RUST", KErrGeneral)` — reported as a panic, category `RUST`,
//!   reason `-2`;
//! - when [`OOM_SWITCH`] exists it asks for more heap than the thread may have, and the
//!   process ends with `User::Exit(KErrNoMemory)` — reported as an exit with `-4`.
//!
//! The index and the allocation size are read at run time (the length of a directory
//! listing and a constant multiplied by it) so the compiler cannot prove the failure and
//! refuse to build, or fold it into something else. Nothing is written: the program has
//! no report, and `symdev test` has nothing to run.
#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use symbian_std::fs;

/// When this file exists the program runs out of memory instead of panicking.
pub const OOM_SWITCH: &str = "E:\\symdev\\panic\\oom";

/// A block no thread heap on this device can hold: the E52 has 128 MiB of RAM, and this
/// is multiplied by at least two below.
const TOO_MUCH: usize = 64 * 1024 * 1024;

#[symbian_std::main]
fn main() -> symbian_std::io::Result<()> {
    let oom = fs::metadata(OOM_SWITCH).is_ok();
    // One more than the entries under `E:\`: a run-time number the optimiser cannot see.
    let past_end = (&fs::read_dir("E:\\")?).into_iter().count() + 1;
    if oom {
        let block: Vec<u8> = alloc::vec![1; TOO_MUCH * (past_end + 1)];
        core::hint::black_box(block);
        return Ok(());
    }
    let three = [1u8, 2, 3];
    // Out of bounds whatever `E:\` holds: the deliberate panic.
    core::hint::black_box(three[past_end + 2]);
    Ok(())
}
