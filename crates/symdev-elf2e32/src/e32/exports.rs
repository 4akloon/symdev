//! `E32Export` / `E32Exports`: a DLL's ordinal table.
use super::header::E32ImageHeaderV;
use crate::{E32DefFile, ElfImage};
use symdev_core::{Error, Result};

/// How an export is typed in the `.def` and `.dso`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E32ExportKind {
    Function,
    /// `DATA <size>` in the `.def`, `STT_OBJECT` of that size in the `.dso`.
    Data(u32),
}

/// One ordinal of a DLL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32Export {
    pub name: String,
    pub ordinal: u32,
    /// Link address; for an `ABSENT` ordinal, the entry point (experiment 54).
    pub address: u32,
    pub kind: E32ExportKind,
    pub absent: bool,
    /// Not in the frozen `.def` (or no `.def` given): listed under `; NEW:`.
    pub new: bool,
    /// The frozen line's comment (from its `;`).
    pub comment: Option<String>,
}

impl E32Export {
    /// Name in the `.dso`: an absent ordinal becomes `_._.absent_export_<n>`
    /// (experiment 54).
    pub fn dso_name(&self) -> String {
        if self.absent {
            format!("_._.absent_export_{}", self.ordinal)
        } else {
            self.name.clone()
        }
    }
}

/// A DLL's exports in ordinal order. Without a frozen `.def`, ordinal = 1-based position
/// after sorting by symbol name (experiment 52: `_Z7MathAbsi` got ordinal 1 despite the
/// highest address). With one, its ordinals are kept and symbols it lacks follow, sorted
/// by name (experiment 54).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32Exports {
    pub entries: Vec<E32Export>,
}

impl E32Exports {
    /// Symbols elf2e32_next leaves out: typeinfo names (experiment 54: `_ZTS6CShape`,
    /// `_ZTS3Foo` and `_ZTSzz` all skipped; `_ZTI`/`_ZTV`/`_ZTT` kept).
    const SKIPPED_PREFIX: &str = "_ZTS";

    /// Exportable symbols: defined global functions and objects in a real section
    /// (experiment 54: `NOTYPE` linker markers and `SHN_ABS` objects never become
    /// exports, with or without `--ignorenoncallable`).
    fn candidates(elf: &ElfImage) -> Result<Vec<(String, u32, E32ExportKind)>> {
        let mut out = Vec::new();
        for sym in elf.exported_symbols()? {
            if sym.is_absolute() || sym.name.starts_with(Self::SKIPPED_PREFIX) {
                continue;
            }
            let kind = if sym.is_function() {
                E32ExportKind::Function
            } else if sym.is_object() {
                E32ExportKind::Data(sym.size)
            } else {
                continue;
            };
            out.push((sym.name, sym.value, kind));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    pub fn from_elf(elf: &ElfImage, frozen: Option<&E32DefFile>) -> Result<Self> {
        let mut candidates = Self::candidates(elf)?;
        let mut entries = Vec::new();
        let mut missing = Vec::new();
        for frozen in frozen.map(|d| d.entries.as_slice()).unwrap_or_default() {
            let kind = match frozen.data_size {
                Some(size) => E32ExportKind::Data(size),
                None => E32ExportKind::Function,
            };
            let address = if frozen.absent {
                if let Some(at) = candidates.iter().position(|c| c.0 == frozen.name) {
                    return Err(Error::Other(format!(
                        "TODO: {} is ABSENT in the .def but defined in the ELF (not observed)",
                        candidates[at].0
                    )));
                }
                elf.entry()
            } else {
                match candidates.iter().position(|c| c.0 == frozen.name) {
                    Some(at) => candidates.remove(at).1,
                    None => {
                        missing.push(frozen.name.clone());
                        continue;
                    }
                }
            };
            entries.push(E32Export {
                name: frozen.name.clone(),
                ordinal: frozen.ordinal,
                address,
                kind,
                absent: frozen.absent,
                new: false,
                comment: frozen.comment.clone(),
            });
        }
        if !missing.is_empty() {
            return Err(Error::Other(format!(
                "frozen export(s) missing from the ELF: {} (mark them ABSENT in the .def to \
                 retire the ordinal)",
                missing.join(", ")
            )));
        }
        for (name, address, kind) in candidates {
            entries.push(E32Export {
                name,
                ordinal: entries.len() as u32 + 1,
                address,
                kind,
                absent: false,
                new: true,
                comment: None,
            });
        }
        Ok(Self { entries })
    }

    /// Export directory appended to the code: `u32 count`, then one link address per
    /// ordinal (experiment 52).
    pub fn table(&self) -> Vec<u8> {
        let mut out = (self.entries.len() as u32).to_le_bytes().to_vec();
        for e in &self.entries {
            out.extend_from_slice(&e.address.to_le_bytes());
        }
        out
    }

    /// `iExportDescType` and `iExportDesc` (experiment 54). No absent ordinal: type 0,
    /// empty. Otherwise a presence bitmap, one bit per ordinal (LSB first, padding bits
    /// set), either whole (type 1) or sparse (type 2: a bitmap of which bytes are not
    /// `0xff`, then those bytes), whichever is shorter.
    pub fn description(&self) -> Result<(u8, Vec<u8>)> {
        if !self.entries.iter().any(|e| e.absent) {
            return Ok((E32ImageHeaderV::EXPORT_DESC_NO_HOLES, Vec::new()));
        }
        let mut full = vec![0xffu8; self.entries.len().div_ceil(8)];
        for (i, e) in self.entries.iter().enumerate() {
            if e.absent {
                full[i / 8] &= !(1 << (i % 8));
            }
        }
        let mut sparse = vec![0u8; full.len().div_ceil(8)];
        let mut holes = Vec::new();
        for (i, &byte) in full.iter().enumerate() {
            if byte != 0xff {
                sparse[i / 8] |= 1 << (i % 8);
                holes.push(byte);
            }
        }
        sparse.extend_from_slice(&holes);
        if full.len() < sparse.len() {
            Ok((E32ImageHeaderV::EXPORT_DESC_FULL_BITMAP, full))
        } else if sparse.len() < full.len() {
            Ok((E32ImageHeaderV::EXPORT_DESC_SPARSE_BITMAP, sparse))
        } else {
            // 9-16 ordinals: both are 2 bytes and elf2e32_next rejects its own image
            // ("gaps between export description and code sections"), so no golden.
            Err(Error::Other(format!(
                "TODO: ABSENT ordinals in a DLL with {} exports (9-16 not observed)",
                self.entries.len()
            )))
        }
    }

    /// `--defoutput` text: frozen lines, then `; NEW:` and the new ones (experiments 52,
    /// 54).
    pub fn def_text(&self) -> String {
        let mut out = String::from("EXPORTS\n");
        let mut in_new = false;
        for e in &self.entries {
            if e.new && !in_new {
                out.push_str("; NEW:\n");
                in_new = true;
            }
            out.push_str(&format!("\t{} @ {} NONAME", e.name, e.ordinal));
            if let E32ExportKind::Data(size) = e.kind {
                out.push_str(&format!(" DATA {size}"));
            }
            if e.absent {
                out.push_str(" ABSENT");
            }
            if let Some(comment) = &e.comment {
                // elf2e32_next writes ` ; ` before the comment it read, `;` included.
                out.push_str(&format!(" ; {comment}"));
            }
            out.push('\n');
        }
        out.push('\n');
        out
    }

    /// Exports not yet in the frozen `.def`.
    pub fn new_names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter(|e| e.new)
            .map(|e| e.name.as_str())
            .collect()
    }
}
