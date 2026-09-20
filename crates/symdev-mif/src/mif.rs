//! The multi-icon container (`.mif`) and its `.mbg` header
//! ([svgb-mif-spec.md](../../../docs/research/svgb-mif-spec.md) §6, §7).

use symdev_core::{Error, Result};

mod depth;
mod icon;

pub use depth::MifDepth;
pub use icon::{MifIcon, MifIconData};

/// A `.mif` file: icons with no alignment or padding between them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MifFile {
    pub icons: Vec<MifIcon>,
}

impl MifFile {
    const HEADER: u32 = 16;
    /// Each icon has two entries, one for it and one for its mask, both
    /// pointing at the same block (spec §6.1).
    const ENTRIES_PER_ICON: u32 = 2;

    pub fn new(icons: Vec<MifIcon>) -> Self {
        Self { icons }
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        if self.icons.is_empty() {
            return Err(Error::Other("MIF without icons".into()));
        }
        let entries = Self::ENTRIES_PER_ICON * self.icons.len() as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"B##4");
        for word in [2, Self::HEADER, entries] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        let mut at = Self::HEADER + 8 * entries;
        let mut blocks = Vec::new();
        for icon in &self.icons {
            let block = icon.block();
            let len = block.as_ref().map_or(0, |b| b.len() as u32);
            for (offset, length) in icon.entries(at, len) {
                out.extend_from_slice(&offset.to_le_bytes());
                out.extend_from_slice(&length.to_le_bytes());
            }
            at += len;
            blocks.extend(block);
        }
        blocks.iter().for_each(|b| out.extend_from_slice(b));
        Ok(out)
    }

    /// `mifconv /H`: the icon enumeration, CRLF throughout (spec §7). The enum and its
    /// enumerators are named after the **header** file's stem (experiment 64:
    /// `games.mif /Hpuzzles_0xa000ef77.mbg` gives `TMifPuzzles_0xa000ef77`). Every icon
    /// takes two values; the `_mask` enumerator appears only when a mask depth was given.
    pub fn mbg_text(&self, header_name: &str) -> String {
        let mif = Self::stem(header_name);
        let mut out = format!(
            " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMif{mif}\r\n\t{{\r\n"
        );
        let mut value = 16384;
        for icon in &self.icons {
            let name = format!("EMbm{mif}{}", Self::stem(&icon.name));
            out.push_str(&format!("\t{name} = {value},\r\n"));
            if icon.has_mask() {
                out.push_str(&format!("\t{name}_mask = {},\r\n", value + 1));
            }
            value += 2;
        }
        out.push_str(&format!("\tEMbm{mif}LastElement\r\n\t}};\r\n"));
        out
    }

    /// File name without its directory and last extension, first character
    /// upper case and the rest lower case (spec §7).
    fn stem(name: &str) -> String {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        let stem = match name.rsplit_once('.') {
            Some((head, _)) => head,
            None => name,
        };
        let mut chars = stem.chars();
        match chars.next() {
            Some(first) => first
                .to_uppercase()
                .chain(chars.flat_map(char::to_lowercase))
                .collect(),
            None => String::new(),
        }
    }
}

#[cfg(test)]
mod tests;
