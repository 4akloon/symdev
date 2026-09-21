//! Symbian paths: a backslash separator and a drive letter as a `Prefix::Disk`.
//!
//! Until this file existed the target shared `unsupported_backslash`, which parses no
//! prefix at all, so `Path::new("C:\\x").is_absolute()` answered `false` and `parent`,
//! `join` and `components` all treated `C:` as an ordinary file name. That was a wrong
//! answer given quietly, which is worse than an error.
//!
//! # What a drive-relative path means here, which is not what it means on Windows
//!
//! `Prefix::Disk` came from Windows, and the temptation is to carry Windows' semantics
//! across with it. **Symbian has no per-drive current directory.** `f32file.h` says so
//! in the `RFs` class documentation itself, in the list of what the file server is for:
//!
//! > maintaining a default path; unlike some other systems, there is a single system
//! > default path, rather than one for each drive: the default path consists of a
//! > drive and a path specification.
//!
//! That single default is the **session path**, one per `RFs` session
//! (`RFs::SessionPath` / `RFs::SetSessionPath`, `f32file.h` lines 1733-1734), and it is
//! what `RFs::Parse` merges an incomplete name against. So `C:x` is still a path this
//! process cannot resolve on its own — it takes the directory from the session path —
//! and `is_absolute` is `false` for it, exactly as on Windows. The *shape* agrees; the
//! reason does not, and the difference is visible: on Windows each drive remembers a
//! directory of its own, so `C:x` and `D:x` can resolve under different directories,
//! while here both take the one session path's directory and differ only in the drive.
//!
//! # No other prefix exists
//!
//! There is no UNC share, no `\\?\` verbatim form and no device namespace on Symbian
//! OS 9.3, so `parse_prefix` recognises a drive and nothing else. A leading `\\` is two
//! empty components of an ordinary rooted path, not a server name.
#![forbid(unsafe_op_in_unsafe_fn)]

mod drive;

use crate::ffi::OsStr;
use crate::io;
use crate::path::{Path, PathBuf, Prefix};
use crate::sys::unsupported;

path_separator_bytes!(b'\\');

/// Symbian has one separator, so a verbatim path — which on Windows is the form that
/// stops taking `/` as one — is not a distinction this platform can make.
#[inline]
pub const fn is_verbatim_sep(b: u8) -> bool {
    is_sep_byte(b)
}

pub fn parse_prefix(path: &OsStr) -> Option<Prefix<'_>> {
    drive::parse_drive(path.as_encoded_bytes()).map(Prefix::Disk)
}

pub const HAS_PREFIXES: bool = true;

/// `std::path::absolute` needs the directory an incomplete path is resolved against,
/// and on Symbian that is the **session path** of an `RFs` session — `RFs::Parse` is
/// the call that does the merge, against a `TParse`.
///
/// Neither is reachable from here yet: `TParse` is a 300-plus-byte class whose layout
/// has not been measured, and reimplementing the merge in Rust would be a guess at
/// rules only `RFs::Parse` states. `TODO: RFs::Parse / TParse (not observed)` — until
/// one of them is, this refuses rather than answering with a path the file server
/// would resolve differently.
pub(crate) fn absolute(_path: &Path) -> io::Result<PathBuf> {
    unsupported()
}

/// A path is absolute when it names a drive *and* starts at that drive's root: `E:\a`
/// is, `E:a` is not (it needs the session path), and `\a` is not (it needs a drive).
pub(crate) fn is_absolute(path: &Path) -> bool {
    path.has_root() && path.prefix().is_some()
}
