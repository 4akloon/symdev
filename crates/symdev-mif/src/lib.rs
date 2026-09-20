//! Symbian icon files: SVG Tiny to binary SVG, and the MIF container.

mod mif;
mod svg;
mod svgb;

pub use mif::{MifFile, MifIcon};
pub use svg::SvgElement;
pub use svgb::Svgb;
