//! The E32 image format: header, sections and export table.
mod code_section;
mod data_section;
mod dll;
mod exports;
mod fixups;
mod header;
mod image;
mod imports;
mod layout;
mod ordinals;
mod reloc_section;
mod target;
mod time;
mod uid;

pub use code_section::E32CodeSection;
pub use data_section::E32DataSection;
pub use dll::E32Dll;
pub use exports::{E32Export, E32ExportKind, E32Exports};
pub use header::{E32ImageHeader, E32ImageHeaderJ, E32ImageHeaderV};
pub use image::E32Image;
pub use imports::{E32ImportBlock, E32ImportSection};
pub use layout::E32Layout;
pub use ordinals::E32Ordinals;
pub use reloc_section::E32RelocSection;
pub use target::E32Target;
pub use time::E32Time;
pub use uid::E32Uid;

#[cfg(test)]
mod tests;
