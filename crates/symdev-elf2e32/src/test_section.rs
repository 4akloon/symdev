//! `TestSection`: one section of a test ELF ([`crate::test_elf::TestElf`]).

/// One section of a [`crate::test_elf::TestElf`]: its type, entry size and contents.
pub(crate) struct TestSection {
    pub kind: u32,
    pub entsize: u32,
    pub data: Vec<u8>,
}
