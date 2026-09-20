//! `ElfImage`: import/local relocations and `.gnu.version_r` DLL versions.
use super::image::ElfImage;
use super::types::{ElfImportReloc, ElfLocalReloc, ElfRel, ElfVersion};
use symdev_core::{Error, Result};

impl ElfImage {
    /// DLL names from `.gnu.version_r`, in section order.
    pub fn needed_dlls(&self) -> Result<Vec<String>> {
        Ok(self.versions()?.into_iter().map(|v| v.dll).collect())
    }

    /// `DT_REL` then `DT_JMPREL` relocations whose symbol is undefined, in file order.
    pub fn import_relocs(&self) -> Result<Vec<ElfImportReloc>> {
        let versym = self
            .section(Self::SHT_GNU_VERSYM)
            .ok_or_else(|| Error::Other("ELF has no .gnu.version".into()))?;
        let versions = self.versions()?;
        let mut out = Vec::new();
        for rel in self.dynamic_relocs()? {
            if rel.symbol == 0 || rel.symbol_section != Self::SHN_UNDEF {
                continue;
            }
            let index = self.u16_at(versym.offset + rel.symbol * 2)? & 0x7fff;
            let version = versions.iter().find(|v| v.index == index).ok_or_else(|| {
                Error::Other(format!(
                    "ELF import at {:#x} has no version {index}",
                    rel.vaddr
                ))
            })?;
            let dynsym = self
                .section(Self::SHT_DYNSYM)
                .ok_or_else(|| Error::Other("ELF has no .dynsym".into()))?;
            out.push(ElfImportReloc {
                dll: version.dll.clone(),
                dso: version.dso.clone(),
                symbol: self.string(dynsym.link, rel.symbol_name)?,
                vaddr: rel.vaddr,
            });
        }
        Ok(out)
    }

    /// Dynamic relocations against defined symbols (the image's own fixups), in file order.
    pub fn local_relocs(&self) -> Result<Vec<ElfLocalReloc>> {
        let mut out = Vec::new();
        for rel in self.dynamic_relocs()? {
            if rel.symbol != 0 && rel.symbol_section == Self::SHN_UNDEF {
                continue;
            }
            if !matches!(
                rel.kind,
                Self::R_ARM_ABS32 | Self::R_ARM_GLOB_DAT | Self::R_ARM_RELATIVE
            ) {
                return Err(Error::Other(format!(
                    "TODO: ARM relocation type {} at {:#x} (not observed)",
                    rel.kind, rel.vaddr
                )));
            }
            out.push(ElfLocalReloc {
                vaddr: rel.vaddr,
                target: rel.symbol_value,
                absolute: rel.kind != Self::R_ARM_RELATIVE,
                addend_in_place: rel.kind == Self::R_ARM_ABS32,
            });
        }
        Ok(out)
    }

    /// `DT_REL` then `DT_JMPREL` entries. `DT_RELSZ` may already span the PLT
    /// relocations (experiment-6 hello.elf); each entry counts once.
    pub(super) fn dynamic_relocs(&self) -> Result<Vec<ElfRel>> {
        let dynsym = self
            .section(Self::SHT_DYNSYM)
            .ok_or_else(|| Error::Other("ELF has no .dynsym".into()))?;
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (start, size) in [
            (Self::DT_REL, Self::DT_RELSZ),
            (Self::DT_JMPREL, Self::DT_PLTRELSZ),
        ] {
            let (Some(at), Some(len)) = (self.dynamic(start)?, self.dynamic(size)?) else {
                continue;
            };
            for rel in self.file_range(at, len)?.step_by(8) {
                if !seen.insert(rel) {
                    continue;
                }
                let info = self.u32_at(rel + 4)?;
                let symbol = (info >> 8) as usize;
                let entry = dynsym.offset + symbol * 16;
                out.push(ElfRel {
                    vaddr: self.u32_at(rel)?,
                    kind: info & 0xff,
                    symbol,
                    symbol_name: self.u32_at(entry)? as usize,
                    symbol_value: self.u32_at(entry + 4)?,
                    symbol_section: self.u16_at(entry + 14)?,
                });
            }
        }
        Ok(out)
    }

    /// Auxiliary entries of `.gnu.version_r`, in section order.
    pub(super) fn versions(&self) -> Result<Vec<ElfVersion>> {
        let Some(verneed) = self.section(Self::SHT_GNU_VERNEED) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        let mut need = verneed.offset;
        loop {
            let count = self.u16_at(need + 2)? as usize;
            let dso = self.string(verneed.link, self.u32_at(need + 4)? as usize)?;
            let mut aux = need + self.u32_at(need + 8)? as usize;
            for _ in 0..count {
                out.push(ElfVersion {
                    index: self.u16_at(aux + 6)?,
                    dll: self.string(verneed.link, self.u32_at(aux + 8)? as usize)?,
                    dso: dso.clone(),
                });
                let next = self.u32_at(aux + 12)? as usize;
                if next == 0 {
                    break;
                }
                aux += next;
            }
            let next = self.u32_at(need + 12)? as usize;
            if next == 0 {
                break;
            }
            need += next;
        }
        Ok(out)
    }
}
