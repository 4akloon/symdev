//! `Mmp` extension methods: derived name and `.dso` library list.
use std::path::Path;

use crate::Mmp;

impl Mmp {
    /// `TARGET` without its extension.
    pub fn name(&self) -> &str {
        Path::new(&self.target)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(self.target.as_str())
    }

    /// `LIBRARY` entries as the `.dso` import libraries GCCE links (`euser.lib` →
    /// `euser.dso`).
    pub fn dso_libraries(&self) -> Vec<String> {
        self.library
            .iter()
            .map(|lib| match lib.rsplit_once('.') {
                Some((stem, ext)) if ext.eq_ignore_ascii_case("lib") => format!("{stem}.dso"),
                _ => lib.clone(),
            })
            .collect()
    }
}
