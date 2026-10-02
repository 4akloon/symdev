//! `ForeignSdkPaths`: what in a Rust project still names another Rust SDK by absolute path.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

use crate::rust_sdk::RustSdk;
use crate::rust_sdk_link::RustSdkLink;

/// The lines of a Rust project's `Cargo.toml` and `.cargo/config.toml` that name, by
/// absolute path, a Rust SDK other than the one the build resolved — what
/// `symdev new --lang rust` wrote before projects named the SDK through
/// [`crate::RustSdkLink`]. Built as it is, such a project would mix the two SDKs.
pub struct ForeignSdkPaths {
    resolved: PathBuf,
    entries: Vec<String>,
}

impl ForeignSdkPaths {
    /// The files looked through, relative to the project root: where the scaffold wrote
    /// the SDK's paths.
    pub const FILES: [&'static str; 2] = ["Cargo.toml", ".cargo/config.toml"];

    /// Looks through the project at `root` for absolute paths into an SDK other than
    /// `sdk`: a `…/crates/<name>` where `sdk` has a crate `<name>`, and a
    /// `…/targets/arm-symbian-e32.json`. A missing file names nothing.
    pub fn find(root: &Path, sdk: &RustSdk) -> Result<ForeignSdkPaths> {
        let crates = Self::crate_names(sdk)?;
        let mut entries = Vec::new();
        for file in Self::FILES {
            let path = root.join(file);
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(Error::Other(format!("{}: {e}", path.display()))),
            };
            let value: toml::Table = text
                .parse()
                .map_err(|e| Error::Other(format!("{}: {e}", path.display())))?;
            let mut strings = Vec::new();
            Self::strings(&toml::Value::Table(value), &mut strings);
            let fixes: Vec<(String, String)> = strings
                .into_iter()
                .filter_map(|s| Self::fix(&s, sdk, &crates).map(|fixed| (s, fixed)))
                .collect();
            Self::entries(file, &text, &fixes, &mut entries);
        }
        Ok(ForeignSdkPaths {
            resolved: sdk.root().to_path_buf(),
            entries,
        })
    }

    /// Fails, naming each line and what to write instead, if there are any. The files
    /// are the developer's: symdev does not rewrite them.
    pub fn check(self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }
        Err(Error::Other(format!(
            "this project names a Rust SDK by absolute path, and not the one this build \
             resolved ({}); symdev links {} to the SDK it builds against, so name the SDK \
             through that link. Change:\n{}",
            self.resolved.display(),
            RustSdkLink::PATH,
            self.entries.join("\n")
        )))
    }

    /// The names of `sdk`'s crates (`crates/<name>`).
    fn crate_names(sdk: &RustSdk) -> Result<Vec<String>> {
        let dir = sdk.root().join("crates");
        let read = std::fs::read_dir(&dir).map_err(|e| {
            Error::Other(format!("Rust SDK has no readable {} ({e})", dir.display()))
        })?;
        Ok(read
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect())
    }

    /// Every string value in `value`, in document order.
    fn strings(value: &toml::Value, out: &mut Vec<String>) {
        match value {
            toml::Value::String(s) => out.push(s.clone()),
            toml::Value::Array(items) => items.iter().for_each(|v| Self::strings(v, out)),
            toml::Value::Table(table) => table.values().for_each(|v| Self::strings(v, out)),
            _ => {}
        }
    }

    /// What the project should write instead of `s`, if `s` is an absolute path into an
    /// SDK other than `sdk`.
    fn fix(s: &str, sdk: &RustSdk, crates: &[String]) -> Option<String> {
        let path = Path::new(s);
        if !path.is_absolute() {
            return None;
        }
        let name = path.file_name()?.to_str()?;
        let parent = path.parent()?;
        let kind = parent.file_name()?;
        let fixed = if kind == "targets" && name == format!("{}.json", RustSdk::TARGET) {
            RustSdkLink::target_spec()
        } else if kind == "crates" && crates.iter().any(|c| c == name) {
            RustSdkLink::crate_dir(name)
        } else {
            return None;
        };
        let named = parent.parent()?;
        let same = named.canonicalize().is_ok_and(|named| named == sdk.root());
        (!same).then_some(fixed)
    }

    /// One entry per line of `text` that holds a path of `fixes`: the line as it is, and
    /// as it should be.
    fn entries(file: &str, text: &str, fixes: &[(String, String)], out: &mut Vec<String>) {
        let mut seen = vec![false; fixes.len()];
        for (number, line) in text.lines().enumerate() {
            let mut fixed = line.to_string();
            for (i, (old, new)) in fixes.iter().enumerate() {
                if fixed.contains(old.as_str()) {
                    fixed = fixed.replace(old.as_str(), new);
                    seen[i] = true;
                }
            }
            if fixed != line {
                out.push(format!(
                    "  {file}:{}\n    - {line}\n    + {fixed}",
                    number + 1
                ));
            }
        }
        // A path TOML spelled with escapes is on no line as it is.
        for ((old, new), seen) in fixes.iter().zip(seen) {
            if !seen {
                out.push(format!("  {file}: `{old}`\n    + `{new}`"));
            }
        }
    }
}

#[cfg(test)]
mod tests;
