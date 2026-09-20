//! `GeneratedCaseFold`: a case-insensitive view of the headers a build generates.
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use super::casefold::include_names;

/// The build writes `Puzzles_0xa000ef77.rsg` (the resource's own spelling) while the
/// sources ask for `puzzles_0xa000ef77.rsg`. On Windows that is the same file; here it
/// is not (`third-party-app-puzzles` gap 10). This is the same trick
/// [`super::SdkIncludeCaseFold`] plays on the SDK headers, applied to `build/`: one
/// symlink per spelling the project's own sources use.
pub struct GeneratedCaseFold;

impl GeneratedCaseFold {
    /// Build the overlay in `out` and return it, for `-I` after the build directory.
    /// `askers` are the files whose `#include` lines decide which spellings are needed
    /// — the project's sources, and its own include directories, which are walked.
    pub fn ensure(build_dir: &Path, askers: &[PathBuf], out: &Path) -> Result<PathBuf> {
        let mut wanted = BTreeSet::new();
        let mut stack: Vec<PathBuf> = askers.to_vec();
        while let Some(asker) = stack.pop() {
            if asker == out {
                continue;
            }
            if asker.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&asker) {
                    stack.extend(entries.flatten().map(|e| e.path()));
                }
                continue;
            }
            if let Ok(bytes) = std::fs::read(&asker) {
                include_names(&bytes, &mut wanted);
            }
        }
        std::fs::create_dir_all(out).map_err(|e| Error::Other(format!("{out:?}: {e}")))?;
        let generated = Self::generated_headers(build_dir)?;
        for name in wanted {
            if name.contains('/') || build_dir.join(&name).exists() {
                continue;
            }
            let Some(real) = generated.get(&name.to_lowercase()) else {
                continue;
            };
            Self::link(real, &out.join(&name))?;
        }
        Ok(out.to_path_buf())
    }

    /// The generated headers directly in `build/`, by lower-cased name.
    fn generated_headers(build_dir: &Path) -> Result<std::collections::BTreeMap<String, PathBuf>> {
        let mut by_lower = std::collections::BTreeMap::new();
        let entries = match std::fs::read_dir(build_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(by_lower),
            Err(e) => return Err(Error::Other(format!("read {build_dir:?}: {e}"))),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if path.is_file() {
                by_lower.insert(name.to_lowercase(), path.clone());
            }
        }
        Ok(by_lower)
    }

    /// Replace any earlier link: a rebuild may have renamed the header it pointed at.
    fn link(real: &Path, link: &Path) -> Result<()> {
        if link.symlink_metadata().is_ok() {
            std::fs::remove_file(link).map_err(|e| Error::Other(format!("{link:?}: {e}")))?;
        }
        Self::symlink(real, link)
    }

    #[cfg(unix)]
    fn symlink(real: &Path, link: &Path) -> Result<()> {
        std::os::unix::fs::symlink(real, link)
            .map_err(|e| Error::Other(format!("symlink {link:?}: {e}")))
    }

    #[cfg(not(unix))]
    fn symlink(_real: &Path, _link: &Path) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_reaches_a_generated_header_spelled_in_another_case() {
        let dir = tempfile::tempdir().unwrap();
        let build = dir.path().join("build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("Puzzles_0xa000ef77.rsg"), b"#define R 1\n").unwrap();
        let src = dir.path().join("main.cpp");
        std::fs::write(
            &src,
            b"#include <puzzles_0xa000ef77.rsg>\n#include <e32base.h>\n",
        )
        .unwrap();
        let out = build.join("generated-casefold");
        let overlay = GeneratedCaseFold::ensure(&build, &[src], &out).unwrap();
        assert_eq!(overlay, out);
        assert_eq!(
            std::fs::read_to_string(out.join("puzzles_0xa000ef77.rsg")).unwrap(),
            "#define R 1\n"
        );
        // Not generated here: the SDK overlay handles those, this one must not guess.
        assert!(!out.join("e32base.h").exists());
    }

    #[test]
    fn a_header_the_build_already_spells_right_gets_no_link() {
        let dir = tempfile::tempdir().unwrap();
        let build = dir.path().join("build");
        std::fs::create_dir_all(&build).unwrap();
        std::fs::write(build.join("hello.rsg"), b"x").unwrap();
        let src = dir.path().join("main.cpp");
        std::fs::write(&src, b"#include <hello.rsg>\n").unwrap();
        let out = build.join("generated-casefold");
        GeneratedCaseFold::ensure(&build, &[src], &out).unwrap();
        assert!(!out.join("hello.rsg").exists());
    }

    #[test]
    fn a_rebuild_repoints_a_stale_link() {
        let dir = tempfile::tempdir().unwrap();
        let build = dir.path().join("build");
        std::fs::create_dir_all(&build).unwrap();
        let src = dir.path().join("main.cpp");
        std::fs::write(&src, b"#include <app.rsg>\n").unwrap();
        let out = build.join("generated-casefold");
        std::fs::write(build.join("App.rsg"), b"first").unwrap();
        GeneratedCaseFold::ensure(&build, std::slice::from_ref(&src), &out).unwrap();
        std::fs::remove_file(build.join("App.rsg")).unwrap();
        std::fs::write(build.join("APP.rsg"), b"second").unwrap();
        GeneratedCaseFold::ensure(&build, &[src], &out).unwrap();
        assert_eq!(
            std::fs::read_to_string(out.join("app.rsg")).unwrap(),
            "second"
        );
    }
}
