//! Where `println!` goes on a phone.
//!
//! # The decision
//!
//! A Symbian phone has no console, so `std`'s three standard streams have to be given
//! somewhere real. The two defensible places are `User::InfoPrint`, the transient note
//! the notifier server flashes on screen, and a file. This platform takes **the file**,
//! at `E:\symdev\stdout.txt`, and here is why:
//!
//! - `println!` is used in loops. `User::InfoPrint` is a *server round trip* that draws
//!   a dialog and holds it for a couple of seconds; a program that printed a hundred
//!   lines through it would be unusable and would not be printing, it would be
//!   interrupting the user.
//! - The file is the channel `symdev` can already read: the emulator's drive E is
//!   `~/.local/share/EKA2L1/data/drives/e/`, which is how `symdev test --emulator`
//!   collects a result file, so `symdev` can show a program's output with no new
//!   machinery. On a device, drive E is the memory card.
//! - Nothing is silently discarded, which was the one option ruled out.
//!
//! What is **not** hidden: a panic still reaches the screen, because
//! `sys::pal::symbian::abort_internal` ends the process with `User::Panic(_L("RUST"),
//! KErrGeneral)` and the platform shows that itself. The message that goes with it is in
//! this file.
//!
//! # What this is not
//!
//! - **Not per process.** One path, shared: two Rust programs running at once interleave
//!   their lines. Naming the file after the application would need `RProcess::FileName`,
//!   which this SDK has not observed, so the honest answer is one well-known path and
//!   this paragraph.
//! - **Not stdin.** There is no console to read from and `Stdin` is always at end of
//!   file; a program that wants input from the user asks the UI for it.
//! - **Not stderr, separately.** Both streams go to the same file, in the order they are
//!   written, which is what a console does.

use crate::fs::{File, OpenOptions};
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, Write};
use crate::sync::OnceLock;

/// Where the standard streams land.
pub const LOG_PATH: &str = "E:\\symdev\\stdout.txt";
/// The directory that has to exist first.
const LOG_DIR: &str = "E:\\symdev";

/// The one open handle, appended to by both streams.
///
/// `None` means the file could not be opened — no drive E, or no room. There is nowhere
/// to report that to (a `print!` returns nothing and must not panic), so a write is
/// dropped and the program carries on, which is what a closed stdout does on every
/// other platform.
static LOG: OnceLock<Option<File>> = OnceLock::new();

fn log() -> Option<&'static File> {
    LOG.get_or_init(|| {
        let _ = crate::fs::create_dir_all(LOG_DIR);
        OpenOptions::new().append(true).create(true).open(LOG_PATH).ok()
    })
    .as_ref()
}

fn append(buf: &[u8]) -> io::Result<usize> {
    match log() {
        Some(mut file) => file.write(buf),
        None => Ok(buf.len()),
    }
}

pub struct Stdin;
pub struct Stdout;
pub struct Stderr;

impl Stdin {
    pub const fn new() -> Stdin {
        Stdin
    }
}

impl io::Read for Stdin {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn read_buf(&mut self, _cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        Ok(())
    }

    fn read_vectored(&mut self, _bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        Ok(0)
    }

    fn is_read_vectored(&self) -> bool {
        false
    }
}

impl Stdout {
    pub const fn new() -> Stdout {
        Stdout
    }
}

impl io::Write for Stdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        append(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(append, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        false
    }

    fn flush(&mut self) -> io::Result<()> {
        match log() {
            Some(mut file) => file.flush(),
            None => Ok(()),
        }
    }
}

impl Stderr {
    pub const fn new() -> Stderr {
        Stderr
    }
}

impl io::Write for Stderr {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        append(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(append, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        false
    }

    fn flush(&mut self) -> io::Result<()> {
        Stdout.flush()
    }
}

/// What `io::stdin` reads ahead into. There is no console, so there is nothing to read
/// ahead.
pub const STDIN_BUF_SIZE: usize = 0;

/// `EBADF` has no Symbian equivalent: a closed handle is `KErrBadHandle`, which
/// [`crate::sys::io::decode_error_kind`] maps to `InvalidInput`.
pub fn is_ebadf(err: &io::Error) -> bool {
    err.raw_os_error() == Some(-8)
}

/// A panic's message goes to the same file. What the *user* sees is the `User::Panic`
/// that ends the process a moment later.
pub fn panic_output() -> Option<impl io::Write> {
    Some(Stderr::new())
}
