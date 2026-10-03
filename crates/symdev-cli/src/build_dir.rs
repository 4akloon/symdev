//! `BuildDir`: a project's `build/`, where every output of a build goes.
use std::path::{Path, PathBuf};

use symdev_core::Error;

/// `build/` in a project: objects, images, cargo's output, the Rust SDK link, and
/// `sdk-include-casefold/`, a tree of symlinks into `SYMDEV_EPOCROOT`'s headers.
pub(crate) struct BuildDir {
    path: PathBuf,
}

impl BuildDir {
    /// The `build/` of the project at `root`.
    pub(crate) fn of(root: &Path) -> BuildDir {
        BuildDir {
            path: root.join("build"),
        }
    }

    /// Creates the directory, invisible to git, before anything is written into it.
    ///
    /// A project's own `.gitignore` is the project's business, and one that does not name
    /// `build/` — any project outside this repository's `examples/` — would commit the
    /// header links with the first `git add -A`. That happened: a branch here once added
    /// 1 044 of them. So the directory ignores itself, the way a build tool's output
    /// directory should. An existing `build/.gitignore` is left alone: if someone wrote
    /// one, it is theirs.
    pub(crate) fn create(&self) -> Result<(), Error> {
        let io = |e: std::io::Error| Error::Other(format!("{}: {e}", self.path.display()));
        std::fs::create_dir_all(&self.path).map_err(io)?;
        let ignore = self.path.join(".gitignore");
        if !ignore.exists() {
            std::fs::write(
                &ignore,
                "# Written by symdev: everything here is build output.\n*\n",
            )
            .map_err(io)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BuildDir;

    #[test]
    fn the_build_directory_ignores_itself() {
        let root = tempfile::tempdir().unwrap();
        BuildDir::of(root.path()).create().unwrap();
        let text = std::fs::read_to_string(root.path().join("build/.gitignore")).unwrap();
        assert!(text.lines().any(|l| l == "*"));
    }

    #[test]
    fn an_existing_ignore_file_is_not_overwritten() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("build")).unwrap();
        std::fs::write(root.path().join("build/.gitignore"), "mine\n").unwrap();
        BuildDir::of(root.path()).create().unwrap();
        let text = std::fs::read_to_string(root.path().join("build/.gitignore")).unwrap();
        assert_eq!(text, "mine\n");
    }
}
