//! Symbian multi-bitmap files: BMP sources, depth conversion, the `.mbm` store.

mod bmp;
mod depth;
mod mbm;
mod rle;

pub use bmp::BmpImage;
pub use depth::MbmDepth;
pub use mbm::{MbmBitmap, MbmFile};
pub use rle::MbmRle;

#[cfg(test)]
mod tests;
