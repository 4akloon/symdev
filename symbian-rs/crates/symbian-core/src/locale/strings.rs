//! Localised strings, read on demand from the nearest-language resource file — the
//! model a C++ application uses, at the same heap cost.
//!
//! symdev compiles `locales/<language>.toml` into `<app>_strings.rsc` and one
//! `<app>_strings.rNN` per language and installs them in `!:\resource\apps\`. Each
//! string is a `BUF8` resource holding the value's UTF-8, and key *i* in byte order is
//! resource index `2 + i` (index 1 is the signature). An application names a string as
//! a [`Str`] constant, which `symbian_std::strings!()` generates from the same files,
//! and asks for its text with [`Str::get`].
//!
//! The first `get` opens the file: `BaflUtils::NearestLanguageFile` picks the variant
//! for the device language, `RResourceFile::OpenL` opens it and `ConfirmSignatureL`
//! reads the NAME offset. That file stays open for the life of the process, as a C++
//! application keeps its resource file open, and costs what C++'s costs (measured
//! +4 cells / +208 B in EKA2L1). Every `get` is then one `AllocReadL`: one heap cell,
//! held by the [`Text`] and freed when it is dropped — nothing else is kept in memory.
//!
//! Each of those calls leaves, so each goes through the C++ shim (`symrs_rsc.cpp`).
use core::cell::{Cell, UnsafeCell};
use core::fmt;
use core::ops::Deref;
use core::ptr::NonNull;

use symbian_sys::bafl::RResourceFile;
use symbian_sys::des::{Lit16, TDesC8_Ptr};
use symbian_sys::des8::{HBufC8, MASK_DES_LENGTH8, TDesC8};
use symbian_sys::euser::User_Free;
use symbian_sys::shim::{symrs_process_file_name, symrs_rsc_open, symrs_rsc_read};

use crate::des::{Buf16, DesC16, PtrC16};
use crate::error::{Result, check};
use crate::fs::with_session;
use crate::{ErrorKind, SymbianError};

/// The low 12 bits of a resource id are the resource's index in its file; the rest is
/// the file's NAME offset (`RResourceFile::Offset()`). A larger index would land in
/// another file's id range, which is never what the generated constants mean.
const MAX_INDEX: u16 = 0x0fff;

/// The two fixed parts of the path, as `_LIT16`s so building it is two `TDes16::Append`s
/// and pulls no UTF-8 → UTF-16 encoder into the program.
const APPS_DIR: Lit16<15> = Lit16::ascii(b"\\resource\\apps\\");
const SUFFIX: Lit16<12> = Lit16::ascii(b"_strings.rsc");

/// One localised string: its resource index in `<app>_strings.rsc`.
///
/// The constants come from `symbian_std::strings!()`, which reads the same
/// `locales/*.toml` symdev compiles, so an index always names the key it was generated
/// for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Str(u16);

impl Str {
    /// The string at resource index `index` (the first string is 2).
    pub const fn at(index: u16) -> Self {
        Self(index)
    }

    /// Reads the string in the device's language (`RResourceFile::AllocReadL`), opening
    /// the strings file on the first call.
    ///
    /// Errors: `KErrNotFound` when no `<app>_strings.*` is installed next to the
    /// application — install the package symdev built, which carries them; `KErrArgument`
    /// for index 0, 1 or above 4095, which no generated constant has; `KErrCorrupt` when
    /// the resource is not UTF-8 — rebuild the strings file with symdev; `KErrInUse` if
    /// called while the file is being opened; `KErrNoMemory` on a full heap.
    pub fn get(self) -> Result<Text> {
        if self.0 < 2 || self.0 > MAX_INDEX {
            return Err(SymbianError::of(ErrorKind::Argument));
        }
        let file = open_file()?;
        let mut cell: *mut HBufC8 = core::ptr::null_mut();
        // SAFETY: `file` points at the process's `RResourceFile`, constructed and opened
        // by the shim and never closed or moved (it is a static); `AllocReadL` is a
        // `const` member, so it only reads it. `cell` is a live out-pointer. The shim is
        // a complete `TRAP` unit, so no exception reaches this frame.
        let code = unsafe { symrs_rsc_read(file, i32::from(self.0), &mut cell) };
        check(code)?;
        let cell = NonNull::new(cell).ok_or(SymbianError::of(ErrorKind::NoMemory))?;
        Text::adopt(cell)
    }
}

/// A string read from the strings file: one heap cell (`HBufC8`), freed on drop.
///
/// It dereferences to `str`; the bytes were checked to be UTF-8 once, when read.
pub struct Text {
    cell: NonNull<HBufC8>,
    bytes: *const u8,
    len: usize,
}

impl Text {
    /// Takes ownership of a cell `AllocReadL` returned and checks its bytes are UTF-8.
    /// On failure the cell is freed here, by `Text`'s own `Drop`.
    fn adopt(cell: NonNull<HBufC8>) -> Result<Self> {
        let header = cell.as_ptr().cast::<u32>();
        // SAFETY: an `HBufC8` is a `TDesC8`, whose first word is the header
        // (`sizeof(TDesC8) == 4`, experiment 79) and whose low 28 bits are the length
        // (`KMaskDesLength8`, `e32des8.h` line 12) — the one field this SDK reads
        // directly. `Ptr()` is euser's own answer to where the bytes are, so the layout
        // of the rest of the cell is not assumed.
        let (len, bytes) = unsafe {
            let len = (header.read() & MASK_DES_LENGTH8) as usize;
            (len, TDesC8_Ptr(cell.as_ptr().cast::<TDesC8>()))
        };
        let text = Self { cell, bytes, len };
        match core::str::from_utf8(text.as_bytes()) {
            Ok(_) => Ok(text),
            Err(_) => Err(SymbianError::of(ErrorKind::Corrupt)),
        }
    }

    fn as_bytes(&self) -> &[u8] {
        // SAFETY: `bytes` and `len` describe the descriptor's contents inside the cell
        // this value owns, which lives until `drop` and is never written after
        // `AllocReadL` returned it.
        unsafe { core::slice::from_raw_parts(self.bytes, self.len) }
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        // SAFETY: `adopt` refused any cell whose bytes are not UTF-8, and they do not
        // change afterwards.
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        // SAFETY: the cell is the `HBufC8` `AllocReadL` allocated on this thread's heap
        // (`e32des8.h`: "hosted by a heap cell") and this value is its only owner.
        // `HBufC8` has no destructor, so freeing the cell is all C++'s `delete` does to
        // it. Checked on the emulator: the cell count returns to where it was.
        unsafe { User_Free(self.cell.as_ptr().cast::<u8>()) }
    }
}

/// Where the process's strings file stands.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Opening,
    Open,
}

/// The process's open strings file, plus the flag that keeps a second open from
/// starting while one is under way.
struct ProcessStrings {
    file: UnsafeCell<RResourceFile>,
    state: Cell<State>,
}

// SAFETY: this is the claim that there is one thread, the same claim and the same
// evidence as the file-server session's (`fs/session.rs`): `core::sync::atomic` does
// not exist on this target (`max-atomic-width: 0`, experiment 72), so no Rust thread
// can be spawned, and this SDK exposes no `RThread`. When threads arrive this has to
// become per-thread or take a lock. Until then the `Cell` is the whole of the
// synchronisation. Rust never makes a reference to `file`: only raw pointers go to the
// shim, which constructs, opens and reads the C++ object in place.
unsafe impl Sync for ProcessStrings {}

static STRINGS: ProcessStrings = ProcessStrings {
    file: UnsafeCell::new(RResourceFile::zeroed()),
    state: Cell::new(State::Closed),
};

/// The open strings file, opening it on first use. Never closed: it lives as long as
/// the process, and the kernel closes its handles when the process ends.
fn open_file() -> Result<*const RResourceFile> {
    match STRINGS.state.get() {
        State::Open => return Ok(STRINGS.file.get()),
        State::Opening => return Err(SymbianError::of(ErrorKind::InUse)),
        State::Closed => {}
    }
    STRINGS.state.set(State::Opening);
    let opened = open_into(STRINGS.file.get());
    STRINGS.state.set(if opened.is_ok() {
        State::Open
    } else {
        State::Closed
    });
    opened.map(|()| STRINGS.file.get().cast_const())
}

/// Opens `<drive>:\resource\apps\<stem>_strings.rsc` — or the variant nearest the
/// device language — into `file`.
fn open_into(file: *mut RResourceFile) -> Result<()> {
    let path = strings_path()?;
    with_session(|fs| {
        // SAFETY: `fs` is the process's session, which lives in a static and is never
        // closed, so a reference `RResourceFile` may keep to it stays valid; `path` is a
        // `TBuf16` borrowed for the call; `file` is the static's 24 bytes, which nothing
        // else touches while the state is `Opening`. The shim is a complete `TRAP` unit
        // and closes the file again on failure.
        let code = unsafe { symrs_rsc_open(fs.as_rfs(), path.as_tdesc16(), file) };
        check(code).map(|_| ())
    })
}

/// `<drive of RProcess::FileName()>\resource\apps\<exe stem>_strings.rsc`.
fn strings_path() -> Result<Buf16<256>> {
    let mut exe = Buf16::<256>::new();
    // SAFETY: the shim copies under `exe`'s own `MaxLength` and answers `KErrOverflow`
    // rather than overflowing; `exe` is a real `TBuf16<256>` alive across the call.
    // `RProcess::FileName` allocates nothing and cannot leave.
    check(unsafe { symrs_process_file_name(exe.as_tdes16()) })?;
    let units = exe.units();
    let bad_name = SymbianError::of(ErrorKind::BadName);
    let drive = units.get(..2).filter(|d| d[1] == u16::from(b':'));
    let drive = drive.ok_or(bad_name)?;
    let name_at = units
        .iter()
        .rposition(|&u| u == u16::from(b'\\'))
        .map_or(0, |i| i + 1);
    let name = &units[name_at..];
    let stem = match name.iter().rposition(|&u| u == u16::from(b'.')) {
        Some(dot) => &name[..dot],
        None => name,
    };
    let mut path = Buf16::<256>::new();
    path.append_des(&PtrC16::new(drive)?)?;
    path.append_des(&APPS_DIR)?;
    path.append_des(&PtrC16::new(stem)?)?;
    path.append_des(&SUFFIX)?;
    Ok(path)
}
