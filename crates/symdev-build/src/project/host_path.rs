//! `HostPath`: opening a DOS-style path on a case-sensitive filesystem.
use std::path::{Component, Path, PathBuf};

/// A path a project file names, looked up on this host.
///
/// `bld.inf` and `.mmp` were written for a case-insensitive filesystem and the SDK
/// itself upper-cases whole `bld.inf` lines before opening the files they name
/// (mmp-frontend-spec.md §2.4, §15 item 6). symdev keeps the source spelling, tries it
/// first, and only then looks for a sibling that differs in case.
pub struct HostPath;

impl HostPath {
    /// `base` + `relative` (`\` or `/`), matching each component case-insensitively when
    /// the exact spelling is missing. `None` when nothing matches.
    pub fn find(base: &Path, relative: &str) -> Option<PathBuf> {
        let mut at = base.to_path_buf();
        for part in Path::new(&relative.replace('\\', "/")).components() {
            match part {
                Component::Normal(name) => {
                    let want = name.to_str()?;
                    let exact = at.join(want);
                    at = if exact.exists() {
                        exact
                    } else {
                        Self::sibling(&at, want)?
                    };
                }
                Component::ParentDir => at.push(".."),
                Component::CurDir => {}
                _ => return None,
            }
        }
        at.exists().then_some(at)
    }

    fn sibling(dir: &Path, want: &str) -> Option<PathBuf> {
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(want))
            })
    }
}
