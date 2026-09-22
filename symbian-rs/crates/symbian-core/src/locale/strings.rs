//! Localised strings, read on demand from the nearest-language resource file — the
//! model a C++ application uses, with no C++ and no `TRAP`.
//!
//! symdev compiles `locales/<language>.toml` into `<app>_strings.rsc` and one
//! `<app>_strings.rNN` per language and installs them in `!:\resource\apps\`. Each
//! string is a `BUF8` resource holding the value's UTF-8, and key *i* in byte order is
//! resource index `2 + i` (index 1 is the signature). An application names a string as
//! a [`Str`] constant, which `symbian_std::strings!()` generates from the same files,
//! and asks for its text with [`Str::get`].
//!
//! The first `get` opens the file: `BaflUtils::NearestLanguageFile` picks the variant
//! for the device language and `RFile::Open` opens it, and [`StringsLayout`] checks the
//! header once and reads the index into one heap cell. That file stays open for the life
//! of the process, as a C++ application keeps its resource file open: the `RFile`
//! handle, two numbers and the index — measured +1 cell / +36 B, against C++'s +4 /
//! +208 B. Every `get` is then one positional `RFile::Read` of the resource's bytes into
//! one heap cell held by the [`Text`]. Holding only the handle and reading the two index
//! entries on each `get` saved that one cell and made a `get` twice as slow (experiment
//! 102), which is why the index is kept.
//!
//! None of it leaves — `RFile` is all `TInt` (`f32file.h`), and `NearestLanguageFile`
//! was followed through the ROM's `bafl.dll` to no leaving call (experiment 102) — so
//! Rust calls it all directly. `RResourceFile`, whose `OpenL`/`AllocReadL` leave, needed
//! a `TRAP` shim and with it the C++ exception runtime.
use core::cell::{Cell, UnsafeCell};

use symbian_sys::bafl::BaflUtils_NearestLanguageFile;
use symbian_sys::des::Lit16;
use symbian_sys::shim::symrs_process_file_name;

use super::heap_bytes::HeapBytes;
use super::layout::{ReadAt, StringsLayout, Unreadable};
use super::text::Text;
use crate::des::{Buf16, DesC16, PtrC16};
use crate::error::{Result, check};
use crate::fs::{File, FileMode, MAX_FILE_NAME, with_session};
use crate::{ErrorKind, SymbianError};

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

    /// Reads the string in the device's language, opening the strings file on the first
    /// call.
    ///
    /// Errors: `KErrNotFound` when no `<app>_strings.*` is installed next to the
    /// application — install the package symdev built, which carries them; `KErrArgument`
    /// for index 0 or 1, which no generated constant has; `KErrCorrupt` when the file is
    /// not one symdev's `rcomp` wrote for strings, the index does not have this resource,
    /// or the resource is not UTF-8 — rebuild with symdev; `KErrInUse` if called while
    /// the file is being opened; `KErrNoMemory` on a full heap.
    pub fn get(self) -> Result<Text> {
        if self.0 < 2 {
            return Err(SymbianError::of(ErrorKind::Argument));
        }
        let open = open_file()?;
        let span = open.layout.span(open.index.as_bytes(), self.0)?;
        Text::new(HeapBytes::read(&open.file, span)?)
    }
}

/// Everything the reader does not accept is a file symdev did not write, so it is
/// `KErrCorrupt`; which case it was is [`Unreadable`]'s, and the host tests name it.
impl From<Unreadable> for SymbianError {
    fn from(_: Unreadable) -> Self {
        SymbianError::of(ErrorKind::Corrupt)
    }
}

impl ReadAt for File {
    type Error = SymbianError;

    fn read_exact_at(&self, at: u32, buf: &mut [u8]) -> Result<()> {
        let wanted = buf.len();
        if self.read_at(at, buf)? == wanted {
            Ok(())
        } else {
            Err(Unreadable::Short {
                at,
                wanted: wanted as u32,
            }
            .into())
        }
    }
}

/// The open strings file: the handle, where things are, and the index — one heap cell of
/// 2(n+1) bytes, so a `get` is one read (C++'s `RResourceFile` holds its index too).
struct OpenStrings {
    file: File,
    layout: StringsLayout,
    index: HeapBytes,
}

/// The process's open strings file, plus the flag that keeps a second open from
/// starting while one is under way.
struct ProcessStrings {
    file: UnsafeCell<Option<OpenStrings>>,
    opening: Cell<bool>,
}

// SAFETY: this is the claim that there is one thread, the same claim and the same
// evidence as the file-server session's (`fs/session.rs`): `core::sync::atomic` does
// not exist on this target (`max-atomic-width: 0`, experiment 72), so no Rust thread
// can be spawned, and this SDK exposes no `RThread`. When threads arrive this has to
// become per-thread or take a lock. Until then the `Cell` is the whole of the
// synchronisation: the slot is written once, from `None` to `Some`, while no reference
// to it exists, and never again.
unsafe impl Sync for ProcessStrings {}

static STRINGS: ProcessStrings = ProcessStrings {
    file: UnsafeCell::new(None),
    opening: Cell::new(false),
};

/// The open strings file, opening it on first use. Never closed: it lives as long as
/// the process, and the kernel closes its handles when the process ends.
fn open_file() -> Result<&'static OpenStrings> {
    let slot = STRINGS.file.get();
    // SAFETY: a shared read of the slot. The only write is below, made while it is
    // still `None` — so before any reference into it was handed out — and it is never
    // written again once `Some`.
    if let Some(open) = unsafe { &*slot } {
        return Ok(open);
    }
    if STRINGS.opening.get() {
        return Err(SymbianError::of(ErrorKind::InUse));
    }
    STRINGS.opening.set(true);
    let file = open_nearest();
    STRINGS.opening.set(false);
    // SAFETY: the slot is `None` (checked above, and nothing between could fill it: the
    // `opening` flag turns any nested call away), so no reference into it exists.
    Ok(unsafe { (*slot).insert(file?) })
}

/// Opens `<drive>:\resource\apps\<stem>_strings.rsc` — or the variant nearest the
/// device language — checks its layout and reads its index.
fn open_nearest() -> Result<OpenStrings> {
    let mut path = strings_path()?;
    let file = with_session(|fs| {
        // SAFETY: `fs` is the process's connected session, borrowed for the call and
        // only read (`const RFs&`); `path` is a real `TBuf16<256>` — a `TFileName`, the
        // exact type the export takes — and `NearestLanguageFile` rewrites it in place
        // within its `iMaxLength`. It does not leave (see the module comment).
        unsafe { BaflUtils_NearestLanguageFile(fs.as_rfs().cast_const(), path.as_tdes16()) };
        File::open_des(fs, &path, FileMode::Read)
    })?;
    let size = u32::try_from(file.size()?).map_err(|_| SymbianError::of(ErrorKind::Corrupt))?;
    let layout = StringsLayout::read(&file, size)?;
    let index = HeapBytes::read(&file, layout.index())?;
    Ok(OpenStrings {
        file,
        layout,
        index,
    })
}

/// `<drive of RProcess::FileName()>\resource\apps\<exe stem>_strings.rsc`.
fn strings_path() -> Result<Buf16<MAX_FILE_NAME>> {
    let mut exe = Buf16::<MAX_FILE_NAME>::new();
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
    let mut path = Buf16::<MAX_FILE_NAME>::new();
    path.append_des(&PtrC16::new(drive)?)?;
    path.append_des(&APPS_DIR)?;
    path.append_des(&PtrC16::new(stem)?)?;
    path.append_des(&SUFFIX)?;
    Ok(path)
}
