//! ELF32 ARM parsing: the parts elf2e32 needs from a linked image.
mod image;
mod raw;
mod relocs;
mod symbols;
mod types;

pub use image::ElfImage;
pub use types::{ElfImportReloc, ElfLocalReloc, ElfSegment, ElfSymbol};
