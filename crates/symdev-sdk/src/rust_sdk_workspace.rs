//! `RustSdkWorkspace`: the Rust SDK's own workspace manifest and the members it names.
use std::path::Path;

use crate::{Result, SdkError};

/// `symbian-rs/Cargo.toml`: the SDK's workspace. Any cargo build inside it — the
/// libcalls build of every Rust application is one (`cargo rustc --manifest-path
/// symbian-rs/crates/symbian-libcalls/Cargo.toml`) — loads **every** member's manifest
/// first, so an SDK whose `examples/` went missing fails inside cargo with "failed to
/// load manifest for workspace member". The SDK checks reject such a tree up front.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustSdkWorkspace {
    members: Vec<String>,
}

impl RustSdkWorkspace {
    /// The manifest's name inside the SDK directory.
    pub const MANIFEST: &'static str = "Cargo.toml";

    /// The workspace of the SDK directory `sdk` (a `symbian-rs`).
    pub fn read(sdk: &Path) -> Result<RustSdkWorkspace> {
        let path = sdk.join(Self::MANIFEST);
        let shown = path.display().to_string();
        let text = std::fs::read_to_string(&path).map_err(|source| SdkError::Io {
            path: shown.clone(),
            source,
        })?;
        let table: toml::Table = text
            .parse()
            .map_err(|e| SdkError::Other(format!("{shown}: {e}")))?;
        let listed = table
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .ok_or_else(|| SdkError::Other(format!("{shown} names no [workspace] members")))?;
        let mut members = Vec::new();
        for member in listed {
            let member = member.as_str().ok_or_else(|| {
                SdkError::Other(format!("{shown}: a [workspace] member is not a string"))
            })?;
            if member.contains(['*', '?', '[']) {
                return Err(SdkError::Other(format!(
                    "TODO: {shown} names the members `{member}` by pattern, and the Rust SDK \
                     has only ever listed them one by one (not observed)"
                )));
            }
            members.push(member.to_string());
        }
        Ok(RustSdkWorkspace { members })
    }

    /// Each member's `Cargo.toml`, relative to the SDK directory, in the manifest's order.
    pub fn member_manifests(&self) -> Vec<String> {
        self.members
            .iter()
            .map(|m| format!("{m}/{}", Self::MANIFEST))
            .collect()
    }

    /// The first member manifest that is not a file under `sdk`, if any.
    pub fn missing_member(&self, sdk: &Path) -> Option<String> {
        self.member_manifests()
            .into_iter()
            .find(|manifest| !sdk.join(manifest).is_file())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::RustSdkWorkspace;
    use crate::SdkError;

    fn sdk_with(manifest: &str, members: &[&str]) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), manifest).unwrap();
        for member in members {
            fs::create_dir_all(tmp.path().join(member)).unwrap();
            fs::write(tmp.path().join(member).join("Cargo.toml"), "").unwrap();
        }
        tmp
    }

    const MANIFEST: &str = "[workspace]\nresolver = \"3\"\n\
                            members = [\"crates/symbian-core\", \"examples/hello\"]\n";

    #[test]
    fn names_each_member_s_manifest_in_order() {
        let sdk = sdk_with(MANIFEST, &[]);
        let workspace = RustSdkWorkspace::read(sdk.path()).unwrap();
        assert_eq!(
            workspace.member_manifests(),
            [
                "crates/symbian-core/Cargo.toml",
                "examples/hello/Cargo.toml"
            ]
        );
    }

    #[test]
    fn finds_the_first_member_that_is_missing() {
        let sdk = sdk_with(MANIFEST, &["crates/symbian-core"]);
        let workspace = RustSdkWorkspace::read(sdk.path()).unwrap();
        let missing = workspace.missing_member(sdk.path());
        assert_eq!(missing.as_deref(), Some("examples/hello/Cargo.toml"));
        let whole = sdk_with(MANIFEST, &["crates/symbian-core", "examples/hello"]);
        assert_eq!(workspace.missing_member(whole.path()), None);
    }

    /// The SDK workspace names its members one by one; a pattern was never observed.
    #[test]
    fn a_member_pattern_is_a_todo() {
        let sdk = sdk_with("[workspace]\nmembers = [\"examples/*\"]\n", &[]);
        let err = RustSdkWorkspace::read(sdk.path()).unwrap_err().to_string();
        assert!(err.starts_with("TODO:"), "{err}");
        assert!(err.contains("examples/*"), "{err}");
    }

    #[test]
    fn a_manifest_without_members_is_named() {
        let sdk = sdk_with("[package]\nname = \"x\"\n", &[]);
        let err = RustSdkWorkspace::read(sdk.path()).unwrap_err().to_string();
        let path = sdk.path().join("Cargo.toml");
        assert_eq!(
            err,
            format!("{} names no [workspace] members", path.display())
        );
    }

    #[test]
    fn a_manifest_that_is_not_toml_is_named() {
        let sdk = sdk_with("{}", &[]);
        let err = RustSdkWorkspace::read(sdk.path()).unwrap_err();
        let path = sdk.path().join("Cargo.toml").display().to_string();
        assert!(err.to_string().starts_with(&format!("{path}: ")), "{err}");
    }

    #[test]
    fn a_missing_manifest_is_an_io_error_naming_it() {
        let sdk = tempfile::tempdir().unwrap();
        let err = RustSdkWorkspace::read(sdk.path()).unwrap_err();
        let path = sdk.path().join("Cargo.toml").display().to_string();
        assert!(
            matches!(&err, SdkError::Io { path: p, .. } if *p == path),
            "{err}"
        );
    }
}
