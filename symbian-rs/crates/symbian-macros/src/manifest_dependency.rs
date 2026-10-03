//! `ManifestDependency`: makes rustc track the application's `symdev.toml`.
//!
//! cargo links through `symdev-ld`, which reads `symdev.toml` (UID3, capabilities, vendor,
//! version, `[ui]`), but cargo does not know that: an edit to the manifest alone would leave
//! the image and the `.sisx` as they were. A discarded `include_str!` of the file, written
//! by `#[symbian_std::main]` and `uid3!()` into the application's crate, puts it in rustc's
//! dep-info, so cargo recompiles and relinks when it changes. The constant is never used,
//! so no byte of it reaches the image.
use std::path::Path;

pub struct ManifestDependency;

impl ManifestDependency {
    /// `const _: &str = include_str!("<dir>/symdev.toml");`, or nothing when `dir` has no
    /// `symdev.toml` (a crate that is not an application).
    pub fn of(dir: &Path) -> String {
        let path = dir.join("symdev.toml");
        match path.is_file() {
            true => format!(
                "const _: &str = ::core::include_str!({:?});\n",
                path.display().to_string()
            ),
            false => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ManifestDependency;

    #[test]
    fn an_existing_manifest_is_named_in_a_discarded_include_str() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(ManifestDependency::of(dir.path()), "");
        std::fs::write(dir.path().join("symdev.toml"), "[symbian]\n").unwrap();
        let got = ManifestDependency::of(dir.path());
        let path = format!("{:?}", dir.path().join("symdev.toml").display().to_string());
        assert_eq!(
            got,
            format!("const _: &str = ::core::include_str!({path});\n")
        );
    }
}
