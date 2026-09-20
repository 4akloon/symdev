//! `MmpPath`: resolving a directory an MMP names, on this host.
use std::path::{Path, PathBuf};

/// A directory named in an MMP (`USERINCLUDE`, `SYSTEMINCLUDE`, `SOURCEPATH`).
///
/// Two kinds, and the difference matters: `..\inc` is relative to the MMP, while
/// `\epoc32\include` is relative to the SDK root — every real MMP carries
/// `SYSTEMINCLUDE \epoc32\include`, and it means the SDK's include directory, never the
/// host's `/epoc32/include` (`third-party-app-puzzles` gap 6).
pub struct MmpPath;

impl MmpPath {
    pub fn resolve(mmp_dir: &Path, epocroot: &Path, dir: &str) -> PathBuf {
        let dir = dir.replace('\\', "/");
        match dir.strip_prefix('/') {
            Some(under_root) => epocroot.join(under_root),
            None => mmp_dir.join(dir),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPOC: &str = "/home/u/sdk/S60_3rd_FP2";

    #[test]
    fn a_relative_directory_hangs_off_the_mmp() {
        assert_eq!(
            MmpPath::resolve(Path::new("/p/group"), Path::new(EPOC), "..\\inc"),
            Path::new("/p/group/../inc")
        );
    }

    #[test]
    fn an_absolute_symbian_directory_hangs_off_the_sdk_root() {
        assert_eq!(
            MmpPath::resolve(Path::new("/p/group"), Path::new(EPOC), "\\epoc32\\include"),
            Path::new(EPOC).join("epoc32/include")
        );
        assert_eq!(
            MmpPath::resolve(
                Path::new("/p/group"),
                Path::new(EPOC),
                "/epoc32/include/stdapis"
            ),
            Path::new(EPOC).join("epoc32/include/stdapis")
        );
    }
}
