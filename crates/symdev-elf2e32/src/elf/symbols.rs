//! `ElfImage`: `.dynsym` lookups (defined symbols, DSO ordinals).
use super::image::ElfImage;
use super::types::ElfSymbol;
use symdev_core::{Error, Result};

impl ElfImage {
    /// Value of a `.dynsym` symbol by exact name.
    pub fn dynamic_symbol(&self, name: &str) -> Result<Option<u32>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(None);
        };
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            if self.string(dynsym.link, self.u32_at(sym)? as usize)? == name {
                return Ok(Some(self.u32_at(sym + 4)?));
            }
        }
        Ok(None)
    }

    /// Defined `STB_GLOBAL` `.dynsym` entries, in `.dynsym` order.
    pub fn exported_symbols(&self) -> Result<Vec<ElfSymbol>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            let info = *self
                .bytes
                .get(sym + 12)
                .ok_or_else(|| Error::Other("ELF .dynsym truncated".into()))?;
            let shndx = self.u16_at(sym + 14)?;
            if shndx == Self::SHN_UNDEF || info >> 4 != Self::STB_GLOBAL {
                continue;
            }
            out.push(ElfSymbol {
                name: self.string(dynsym.link, self.u32_at(sym)? as usize)?,
                value: self.u32_at(sym + 4)?,
                size: self.u32_at(sym + 8)?,
                kind: info & 0xf,
                section: shndx,
            });
        }
        Ok(out)
    }

    /// For a Symbian DSO: the ordinal an exported symbol names, i.e. the word its
    /// `.dynsym` value points at in its section (experiment 44: `ER_RO` ordinal table).
    pub fn dso_ordinal(&self, name: &str) -> Result<Option<u32>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(None);
        };
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            let shndx = self.u16_at(sym + 14)?;
            if shndx == Self::SHN_UNDEF
                || self.string(dynsym.link, self.u32_at(sym)? as usize)? != name
            {
                continue;
            }
            let section = self
                .sections
                .get(shndx as usize)
                .ok_or_else(|| Error::Other(format!("DSO symbol {name} in missing section")))?;
            let value = self.u32_at(sym + 4)?;
            let at = value
                .checked_sub(section.addr)
                .ok_or_else(|| Error::Other(format!("DSO symbol {name} below its section")))?;
            return Ok(Some(self.u32_at(section.offset + at as usize)?));
        }
        Ok(None)
    }
}
