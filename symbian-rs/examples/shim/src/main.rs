//! The C++ shim (experiment 78, design spec §7 step 70): what a leaving Symbian call
//! looks like from Rust, next to a non-leaving one that needs no shim at all.
//!
//! Three things happen, and the point of the example is that all three are visible in
//! one notifier line — which can only be true if the leave was caught:
//!
//! 1. `FileServer::connect` and `make_dir_all` are **non-static member functions of
//!    `RFs` that cannot leave**, so Rust calls them directly with `this` as argument 0
//!    and there is no C++ on the stack.
//! 2. `shim::leave_if_error(-12)` **really leaves**: `User::LeaveIfError` raises the
//!    exception inside the C++ frame, the shim's `TRAP` catches it, and -12 arrives here
//!    as an ordinary `Err`. Untrapped, the process would vanish at this line with
//!    nothing in the log (experiment 76, probe A−). `ensure_path_exists` is the same
//!    mechanism over the real file API, `BaflUtils::EnsurePathExistsL`.
//! 3. The report is built with `Buf16::append`/`append_num`, euser's own `TDes16`
//!    members, so this binary links none of `core::fmt`.
#![no_std]
#![no_main]

use symbian_core::{Buf16, FileServer, Result, shim, user};

/// A directory the emulator's drive E really has.
const GOOD: &str = "E:\\symdev\\shim70\\out.txt";
/// A drive that is not mounted, so `EnsurePathExistsL` leaves.
const BAD: &str = "Y:\\symdev\\shim70\\out.txt";

fn path(text: &str) -> Result<Buf16<64>> {
    let mut buf = Buf16::new();
    buf.push_str(text)?;
    Ok(buf)
}

/// The raw `TInt` of a call, so the note shows `KErrNone` as 0 and a failure as itself.
fn code(result: Result<()>) -> i64 {
    match result {
        Ok(()) => 0,
        Err(e) => i64::from(e.code()),
    }
}

fn run() -> Result<()> {
    let mut note = Buf16::<160>::new();
    let mut fs = FileServer::connect()?;

    // No shim: `RFs::MkDirAll` cannot leave and reports its own error.
    note.push_str("shim70 mkdirall=")?;
    note.append_num(code(fs.make_dir_all(&path(GOOD)?)))?;

    // Through the shim, and this one really leaves: `User::LeaveIfError(-12)` raises a
    // C++ exception inside the C++ frame, the `TRAP` catches it, and -12 arrives here.
    // Untrapped, the process would end at this line with nothing in the log.
    note.push_str(" trapped=")?;
    note.append_num(code(shim::leave_if_error(-12)))?;

    // The real file-domain wrapper, on both a bad path and a good one. EKA2L1's file
    // server accepts paths a phone would refuse, so these are expected to succeed here;
    // what they show is that the trapped path is also the ordinary path.
    note.push_str(" bad=")?;
    note.append_num(code(fs.ensure_path_exists(&path(BAD)?)))?;
    note.push_str(" ensured=")?;
    note.append_num(code(fs.ensure_path_exists(&path(GOOD)?)))?;

    // A negative number, to show how euser renders the sign.
    note.push_str(" sign=")?;
    note.append_num(-42)?;
    note.push_str(" alive")?;

    user::info_print(&note)?;
    user::after(5_000_000);
    Ok(())
}

fn main() -> i32 {
    match run() {
        Ok(()) => 0,
        Err(e) => e.code(),
    }
}

symbian_runtime::entry!(main);
