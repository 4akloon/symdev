//! Small value types read out of an ELF image: segments, relocations, symbols.

/// One `PT_LOAD` program header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfSegment {
    pub vaddr: u32,
    pub offset: u32,
    pub file_size: u32,
    pub mem_size: u32,
}

/// A dynamic relocation against an undefined symbol, tagged with the DLL (and the
/// DSO file) that `.gnu.version_r` says provides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfImportReloc {
    pub dll: String,
    pub dso: String,
    pub symbol: String,
    pub vaddr: u32,
}

/// A dynamic relocation against a defined symbol: the word at `vaddr` refers to `target`.
/// `absolute` words are rewritten to the symbol address: `R_ARM_ABS32` adds the
/// word already in place (`S + A`), `R_ARM_GLOB_DAT` does not (`S`, `addend_in_place`
/// false). `R_ARM_RELATIVE` words already hold the link-time address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfLocalReloc {
    pub vaddr: u32,
    pub target: u32,
    pub absolute: bool,
    pub addend_in_place: bool,
}

/// One `.gnu.version_r` auxiliary entry.
#[derive(Debug, Clone)]
pub(super) struct ElfVersion {
    pub(super) index: u16,
    pub(super) dll: String,
    pub(super) dso: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ElfRel {
    pub(super) vaddr: u32,
    pub(super) kind: u32,
    pub(super) symbol: usize,
    pub(super) symbol_name: usize,
    pub(super) symbol_value: u32,
    pub(super) symbol_section: u16,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ElfSection {
    pub(super) kind: u32,
    pub(super) addr: u32,
    pub(super) offset: usize,
    pub(super) size: usize,
    pub(super) link: usize,
}

/// A defined global `.dynsym` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSymbol {
    pub name: String,
    pub value: u32,
    pub size: u32,
    /// `STT_*`.
    pub kind: u8,
    /// `st_shndx`.
    pub section: u16,
}

impl ElfSymbol {
    const STT_OBJECT: u8 = 1;
    const STT_FUNC: u8 = 2;
    const SHN_ABS: u16 = 0xfff1;

    pub fn is_function(&self) -> bool {
        self.kind == Self::STT_FUNC
    }

    pub fn is_object(&self) -> bool {
        self.kind == Self::STT_OBJECT
    }

    /// `SHN_ABS` (linker-made markers such as the version-name symbol).
    pub fn is_absolute(&self) -> bool {
        self.section == Self::SHN_ABS
    }
}
