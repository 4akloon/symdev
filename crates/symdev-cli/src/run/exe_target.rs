//! `ExeTarget`: what cargo hands the runner — an image `symdev-ld` linked — and what to
//! install for it (design spec §6.1; experiment 114 §1.1: `cargo run` passes the hard link
//! `<profile>/<bin>`, relative to the working directory; `cargo test` the absolute `-o`).
use std::io::Read;
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::ld::{LinkKind, LinkRecord};

/// UID1 of an E32 executable (`KExecutableImageUid`).
const EXE_UID1: u32 = 0x1000_007a;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExeTarget {
    pub image: PathBuf,
    pub sisx: PathBuf,
    pub uid3: u32,
    pub kind: LinkKind,
}

impl ExeTarget {
    /// `exe` as cargo passed it, resolved against `cwd`.
    pub fn of(exe: &Path, cwd: &Path) -> Result<Self> {
        let image = cwd.join(exe);
        let mut header = [0u8; 12];
        std::fs::File::open(&image)
            .and_then(|mut f| f.read_exact(&mut header))
            .map_err(|e| Error::Other(format!("{}: {e}", image.display())))?;
        let word = |at: usize| {
            u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
        };
        if word(0) != EXE_UID1 {
            return Err(Error::Other(format!(
                "{} is not an E32 executable (UID1 0x{:08x}); symdev run --exe takes what \
                 symdev-ld linked",
                image.display(),
                word(0)
            )));
        }
        let sisx = with_suffix(&image, ".sisx");
        if !sisx.is_file() {
            return Err(Error::Other(format!(
                "{} has no {} beside it: it was not linked by symdev-ld; check that \
                 .cargo/config.toml names linker = \"symdev-ld\" for arm-symbian-e32",
                image.display(),
                sisx.display()
            )));
        }
        if modified(&sisx)? < modified(&image)? {
            return Err(Error::Other(format!(
                "{} is older than {}: the last link stopped after writing the image; run \
                 cargo build again",
                sisx.display(),
                image.display()
            )));
        }
        let kind = LinkRecord::read(&with_suffix(&image, ".symdev.toml"))?.kind;
        Ok(Self {
            image,
            sisx,
            uid3: word(8),
            kind,
        })
    }

    /// The project's own package `build/<name>.sisx`, for `symdev run` and `symdev test`
    /// without `--exe`. A package older than the `build/<name>.exe` beside it would
    /// install the previous build, so it is refused.
    pub fn installed(sisx: PathBuf, uid3: u32) -> Result<Self> {
        let name = sisx
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !sisx.is_file() {
            return Err(Error::Other(format!(
                "SISX not found: build/{name} (run symdev package; for a Rust project, cargo build)"
            )));
        }
        let image = sisx.with_extension("exe");
        if image.is_file() && modified(&sisx)? < modified(&image)? {
            return Err(Error::Other(format!(
                "build/{name} is older than {}, so this would install the previous build: run                  symdev package (or cargo build for a Rust project)",
                image.display()
            )));
        }
        Ok(Self {
            image,
            sisx,
            uid3,
            kind: LinkKind::Main,
        })
    }
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn modified(path: &Path) -> Result<std::time::SystemTime> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map_err(|e| Error::Other(format!("{}: {e}", path.display())))
}
