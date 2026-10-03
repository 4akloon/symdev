//! `LinkRecord`: `<out>.symdev.toml`, what `symdev-ld` tells the runner about an image.
use std::path::Path;

use symdev_core::{Error, Result};

use super::LinkKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkRecord {
    pub kind: LinkKind,
}

impl LinkRecord {
    pub fn write(&self, path: &Path) -> Result<()> {
        let text = match &self.kind {
            LinkKind::Main => "kind = \"main\"\n".to_string(),
            LinkKind::Test { name } => format!("kind = \"test\"\nname = {:?}\n", name),
        };
        std::fs::write(path, text).map_err(|e| Error::Other(format!("{}: {e}", path.display())))
    }

    #[allow(dead_code)] // Task 13: the runner reads the record
    pub fn read(path: &Path) -> Result<Self> {
        let bad = |why: &str| {
            Error::Other(format!(
                "{}: {why}; relink with cargo build",
                path.display()
            ))
        };
        let text = std::fs::read_to_string(path).map_err(|e| bad(&e.to_string()))?;
        let table: toml::Table = text
            .parse()
            .map_err(|e: toml::de::Error| bad(&e.to_string()))?;
        match (
            table.get("kind").and_then(|v| v.as_str()),
            table.get("name").and_then(|v| v.as_str()),
        ) {
            (Some("main"), _) => Ok(Self {
                kind: LinkKind::Main,
            }),
            (Some("test"), Some(name)) => Ok(Self {
                kind: LinkKind::Test { name: name.into() },
            }),
            _ => Err(bad("not a symdev-ld record")),
        }
    }
}
