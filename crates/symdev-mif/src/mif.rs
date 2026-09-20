//! The multi-icon container (`.mif`) and its `.mbg` header
//! ([svgb-mif-spec.md](../../../docs/research/svgb-mif-spec.md) §6, §7).

use symdev_core::{Error, Result};

/// One icon of a `.mif`: its encoded data and how it is displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MifIcon {
    /// Source file stem, for the `.mbg` enumerators.
    pub name: String,
    /// The `.svgb` bytes.
    pub data: Vec<u8>,
    /// Display mode (`/c32,8` → 11 with mask 4).
    pub depth: u32,
    pub mask_depth: u32,
    pub animated: bool,
}

impl MifIcon {
    /// `mifconv /c32,8`, the depth the SDK example icon makefiles use.
    pub const DEPTH_COLOUR32: u32 = 11;
    pub const MASK_8BIT: u32 = 4;
    const SVG: u32 = 1;
    const HEADER: usize = 32;

    pub fn svg(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data,
            depth: Self::DEPTH_COLOUR32,
            mask_depth: Self::MASK_8BIT,
            animated: false,
        }
    }

    fn block(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER + self.data.len());
        out.extend_from_slice(b"C##4");
        for word in [
            1,
            Self::HEADER as u32,
            self.data.len() as u32,
            Self::SVG,
            self.depth,
            u32::from(self.animated),
            self.mask_depth,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(&self.data);
        out
    }
}

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
            for _ in 0..Self::ENTRIES_PER_ICON {
                out.extend_from_slice(&at.to_le_bytes());
                out.extend_from_slice(&(block.len() as u32).to_le_bytes());
            }
            at += block.len() as u32;
            blocks.push(block);
        }
        blocks.iter().for_each(|b| out.extend_from_slice(b));
        Ok(out)
    }

    /// `mifconv /H`: the icon enumeration, CRLF throughout (spec §7).
    pub fn mbg_text(&self, mif_name: &str) -> String {
        let mif = Self::stem(mif_name);
        let mut out = format!(
            " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMif{mif}\r\n\t{{\r\n"
        );
        let mut value = 16384;
        for icon in &self.icons {
            let name = format!("EMbm{mif}{}", Self::stem(&icon.name));
            out.push_str(&format!("\t{name} = {value},\r\n"));
            out.push_str(&format!("\t{name}_mask = {},\r\n", value + 1));
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
