//! `CargoLinkEnv`: the variables cargo gives rustc, and rustc its linker (exp. 114 §1.1).
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CargoLinkEnv {
    pub bin_name: Option<String>,
    pub crate_name: Option<String>,
    pub target_tmpdir: Option<String>,
    pub manifest_dir: Option<PathBuf>,
}

impl CargoLinkEnv {
    /// From `(name, value)` pairs — `std::env::vars()` in `LinkRun`, a fixture in tests.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut env = Self::default();
        for (k, v) in pairs {
            match k.as_str() {
                "CARGO_BIN_NAME" => env.bin_name = Some(v),
                "CARGO_CRATE_NAME" => env.crate_name = Some(v),
                "CARGO_TARGET_TMPDIR" => env.target_tmpdir = Some(v),
                "CARGO_MANIFEST_DIR" => env.manifest_dir = Some(PathBuf::from(v)),
                _ => {}
            }
        }
        env
    }
}
