//! `mifconv /OPT`: `DEPTH[,MASK]` and the display modes it stands for
//! ([svgb-mif-spec.md](../../../../docs/research/svgb-mif-spec.md) §3, §6.2).

use symdev_core::{Error, Result};

/// One icon's `/OPT`: the depth token as `bmconv` spells it and the mask depth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MifDepth {
    /// `1`, `2`, `4`, `8`, `c4`, `c8`, `c12`, `c16`, `c24` or `c32`, lower case.
    pub depth: String,
    /// `1` or `8`; `None` when no mask depth was given.
    pub mask: Option<u8>,
}

impl MifDepth {
    /// Depth token to the display mode written at offset 20 of the icon header.
    const MODES: &[(&str, u32)] = &[
        ("1", 1),
        ("2", 2),
        ("4", 3),
        ("8", 4),
        ("c4", 5),
        ("c8", 6),
        ("c12", 10),
        ("c16", 7),
        ("c24", 8),
        ("c32", 11),
    ];

    /// `c32,8` as it stands after `/`, case-insensitive.
    pub fn parse(text: &str) -> Result<Self> {
        let text = text.to_ascii_lowercase();
        let (depth, mask) = match text.split_once(',') {
            Some((depth, mask)) => (depth, Some(mask)),
            None => (text.as_str(), None),
        };
        if !Self::MODES.iter().any(|(token, _)| *token == depth) {
            return Err(Error::Other(format!(
                "icon depth `{text}`: the depth must be one of 1, 2, 4, 8, c4, c8, c12, c16, \
                 c24, c32, optionally followed by `,1` or `,8` for the mask"
            )));
        }
        let mask = match mask {
            None => None,
            Some("1") => Some(1),
            Some("8") => Some(8),
            Some(other) => {
                return Err(Error::Other(format!(
                    "icon depth `{text}`: the mask depth `{other}` must be 1 or 8"
                )));
            }
        };
        Ok(Self {
            depth: depth.to_string(),
            mask,
        })
    }

    /// The icon header's display mode (spec §6.2).
    pub fn display_mode(&self) -> u32 {
        Self::MODES
            .iter()
            .find(|(token, _)| *token == self.depth)
            .map_or(0, |(_, mode)| *mode)
    }

    /// The icon header's mask display mode: 0 without a mask, 1 for `,1`, 4 for `,8`.
    pub fn mask_mode(&self) -> u32 {
        match self.mask {
            None => 0,
            Some(1) => 1,
            Some(_) => 4,
        }
    }
}
