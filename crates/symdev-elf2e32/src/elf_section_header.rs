//! `ElfSectionHeader`: what an in-place patch needs of one section header.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ElfSectionHeader {
    pub index: usize,
    pub kind: u32,
    pub offset: usize,
    pub size: usize,
    pub entsize: usize,
}
