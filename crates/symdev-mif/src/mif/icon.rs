//! One icon of a `.mif`: an SVG carried in the file, or a bitmap kept in the sibling
//! `.mbm` ([svgb-mif-spec.md](../../../../docs/research/svgb-mif-spec.md) §6.1,
//! experiment 64 for the bitmap entries).

use crate::mif::depth::MifDepth;

/// Where an icon's pixels live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MifIconData {
    /// The `.svgb` bytes, written as a block of the `.mif`.
    Svg {
        data: Vec<u8>,
        /// Display mode (`/c32,8` → 11 with mask 4).
        depth: u32,
        mask_depth: u32,
        animated: bool,
    },
    /// A bitmap of the sibling `.mbm`: the entry carries the negated index and no
    /// block. A mask is a bitmap of its own, right after the icon.
    Bitmap { index: u32, mask: Option<u32> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MifIcon {
    /// Source file name, for the `.mbg` enumerators.
    pub name: String,
    pub data: MifIconData,
}

impl MifIcon {
    /// `mifconv /c32,8`, the depth the SDK example icon makefiles use.
    pub const DEPTH_COLOUR32: u32 = 11;
    pub const MASK_8BIT: u32 = 4;
    const SVG: u32 = 1;
    const HEADER: usize = 32;

    /// An SVG icon at `/c32,8`.
    pub fn svg(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            data: MifIconData::Svg {
                data,
                depth: Self::DEPTH_COLOUR32,
                mask_depth: Self::MASK_8BIT,
                animated: false,
            },
        }
    }

    /// An SVG icon at any `/OPT`, `animated` for `/A`.
    pub fn svg_at(name: impl Into<String>, data: Vec<u8>, at: &MifDepth, animated: bool) -> Self {
        Self {
            name: name.into(),
            data: MifIconData::Svg {
                data,
                depth: at.display_mode(),
                mask_depth: at.mask_mode(),
                animated,
            },
        }
    }

    /// A bitmap icon: `index` of the icon in the `.mbm`, `mask` that of its mask.
    pub fn bitmap(name: impl Into<String>, index: u32, mask: Option<u32>) -> Self {
        Self {
            name: name.into(),
            data: MifIconData::Bitmap { index, mask },
        }
    }

    /// Whether the `.mbg` lists a `_mask` enumerator for this icon.
    pub fn has_mask(&self) -> bool {
        match &self.data {
            MifIconData::Svg { mask_depth, .. } => *mask_depth != 0,
            MifIconData::Bitmap { mask, .. } => mask.is_some(),
        }
    }

    /// The icon block for an SVG icon; a bitmap icon has none.
    pub(super) fn block(&self) -> Option<Vec<u8>> {
        let MifIconData::Svg {
            data,
            depth,
            mask_depth,
            animated,
        } = &self.data
        else {
            return None;
        };
        let mut out = Vec::with_capacity(Self::HEADER + data.len());
        out.extend_from_slice(b"C##4");
        for word in [
            1,
            Self::HEADER as u32,
            data.len() as u32,
            Self::SVG,
            *depth,
            u32::from(*animated),
            *mask_depth,
        ] {
            out.extend_from_slice(&word.to_le_bytes());
        }
        out.extend_from_slice(data);
        Some(out)
    }

    /// The two table entries, `(offset, length)`: the icon's and its mask's.
    pub(super) fn entries(&self, block_at: u32, block_len: u32) -> [(u32, u32); 2] {
        match &self.data {
            MifIconData::Svg { .. } => [(block_at, block_len); 2],
            MifIconData::Bitmap { index, mask } => [
                (index.wrapping_neg(), 0),
                (mask.unwrap_or(*index).wrapping_neg(), 0),
            ],
        }
    }
}
