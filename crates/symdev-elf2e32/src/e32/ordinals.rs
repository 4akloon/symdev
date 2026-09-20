//! `E32Ordinals`: import ordinals by `(dll, symbol)`.

/// Import ordinals by `(dll, symbol)`, as the `--libpath` DSOs define them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct E32Ordinals {
    map: std::collections::BTreeMap<(String, String), u32>,
}

impl E32Ordinals {
    pub fn insert(&mut self, dll: impl Into<String>, symbol: impl Into<String>, ordinal: u32) {
        self.map.insert((dll.into(), symbol.into()), ordinal);
    }

    pub fn get(&self, dll: &str, symbol: &str) -> Option<u32> {
        self.map
            .get(&(dll.to_string(), symbol.to_string()))
            .copied()
    }
}
