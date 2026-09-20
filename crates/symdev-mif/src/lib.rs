//! Symbian icon files: SVG Tiny to binary SVG, and the MIF container.

mod mif;
mod svg;
mod svgb;

pub use mif::{MifDepth, MifFile, MifIcon, MifIconData};
pub use svg::SvgElement;
pub use svgb::Svgb;
