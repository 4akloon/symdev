//! The image `examples/std-hello` spawns, and nothing else.
//!
//! It reads the command line its creator gave it, writes it to [`MARK`] for the
//! creator to read back, and ends with that number as its exit code. Between the two
//! programs that proves the whole of what `std::process` can do on this platform:
//! `RProcess::Create` + `Resume` start it, the command line survives the trip, and
//! `RProcess::Logon` + `User::WaitForRequest` hand the code back.
//!
//! # Why the child is `#![no_std]` while the parent is not
//!
//! **EKA2L1 cannot spawn an image with a writable data section through the loader.**
//! `RProcess::Create` succeeds, the emulator gives the new process an extra
//! `anonymous` chunk of 0x1000 bytes for its data at 0x400000, and the child then dies
//! with `KERN-EXEC 3` before it reaches `main` — reading its own heap base + 0xA4. It
//! happens for every `std` image tried (which all have a data section) and for none of
//! the `no_std` ones (whose `runtime data` the emulator logs as `0x0`). A `no_std`
//! child is therefore the only child that runs here, and it makes the point just as
//! well: what is under test is the **parent's** `std::process`, and this proves it can
//! launch any Symbian executable.
//!
//! Install it before running `std-hello`, the way `examples/net` needs its two peers:
//!
//! ```sh
//! cd symbian-rs/examples/spawnee && symdev build && symdev package && symdev run
//! ```
#![no_std]

use symbian_std::fs;
use symbian_sys::des16::{TPtr16_ctor, TPtr16Storage};
use symbian_sys::euser::{User_CommandLine, User_CommandLineLength, User_Exit};

/// Where the child writes the command line it was given.
pub const MARK: &str = "E:\\symdev\\spawnee\\args.txt";

/// The longest command line this program will read. Its creator sends one number.
const MAX: usize = 64;

#[symbian_std::main]
fn main() -> symbian_std::io::Result<()> {
    let mut units = [0u16; MAX];
    let mut des = TPtr16Storage::zeroed();
    // SAFETY: `TPtr16::TPtr16(TUint16*, TInt, TInt)` is built in place in storage of
    // the measured `sizeof(TPtr16)` (12), 4-aligned as measured, over this frame's own
    // array — so euser writes the descriptor header and nothing here guesses it. The
    // length starts at zero so that `User::CommandLine` fills it from the start.
    unsafe { TPtr16_ctor(&mut des, units.as_mut_ptr(), 0, MAX as i32) };
    // SAFETY: two euser statics. `CommandLineLength` takes nothing; `CommandLine`
    // takes the `TDes16&` just built, whose `iMaxLength` is `MAX`, and it is only
    // called when the line fits — an overflow would be `USER 11`, a panic no `TRAP`
    // catches.
    let len = unsafe { User_CommandLineLength() };
    if len > 0 && len as usize <= MAX {
        unsafe { User_CommandLine(des.as_tdes16()) };
    }

    // The creator sends ASCII digits, so the narrowing is lossless for anything this
    // program is meant to be given; anything else lands as a byte it will not parse.
    let mut text = [0u8; MAX];
    let read = des.length().min(MAX);
    for (byte, unit) in text[..read].iter_mut().zip(&units[..read]) {
        *byte = if *unit < 0x80 { *unit as u8 } else { b'?' };
    }
    let line = core::str::from_utf8(&text[..read]).unwrap_or("");

    let _ = fs::create_dir_all("E:\\symdev\\spawnee");
    let _ = fs::write(MARK, line.as_bytes());

    // `User::Exit(TInt)` is what the creator reads back as `RProcess::ExitReason()`.
    // SAFETY: a euser static taking one scalar; it never returns, which is why nothing
    // follows it.
    unsafe { User_Exit(line.trim().parse::<i32>().unwrap_or(0)) }
}
