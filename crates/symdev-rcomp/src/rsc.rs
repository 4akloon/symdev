//! `.rsc` (`rcomp -u`) and `.rsg` output for a compiled resource file (experiment 56).

use symdev_core::{Error, Result};
use symdev_uidcrc::UidCrc;

use crate::compiler::RscCompiled;
use crate::pack::RscPacker;

impl RscCompiled {
    pub const UID1: u32 = 0x101f_4a6b;
    /// Header flag: UID3 is the `NAME` value (no `UID3` statement).
    pub const FLAG_UID3_FROM_NAME: u8 = 0x01;

    /// UIDs, flags, largest uncompressed resource (`u16`), one bit per resource that is
    /// packed, the resources, then `u16` start offsets and the index offset.
    pub fn rsc_bytes(&self) -> Result<Vec<u8>> {
        let mut out = UidCrc::new(Self::UID1, self.uid2, self.uid3)
            .bytes()
            .to_vec();
        out.push(if self.uid3_from_name {
            Self::FLAG_UID3_FROM_NAME
        } else {
            0
        });
        let largest = self
            .resources
            .iter()
            .map(|r| r.data.len())
            .max()
            .unwrap_or(0);
        out.extend_from_slice(
            &u16::try_from(largest)
                .map_err(|_| Error::Other(format!("resource of {largest} bytes")))?
                .to_le_bytes(),
        );
        let mut bits = vec![0u8; self.resources.len().div_ceil(8)];
        let mut bodies = Vec::new();
        for (i, r) in self.resources.iter().enumerate() {
            match RscPacker::pack(&r.data)? {
                Some(packed) => {
                    bits[i / 8] |= 1 << (i % 8);
                    bodies.push(packed);
                }
                None => bodies.push(r.data.uncompressed()),
            }
        }
        out.extend_from_slice(&bits);
        let mut index = Vec::new();
        for body in bodies {
            index.push(out.len());
            out.extend_from_slice(&body);
        }
        index.push(out.len());
        for at in index {
            out.extend_from_slice(
                &u16::try_from(at)
                    .map_err(|_| Error::Other(format!("resource file over 64 KB ({at})")))?
                    .to_le_bytes(),
            );
        }
        Ok(out)
    }

    /// `rcomp -h`: `#define <NAME>` padded to column 50 (at least one space), the id
    /// as `0x%x` (decimal without a `NAME` statement), CRLF, for each named resource
    /// (experiment 56).
    pub fn rsg_text(&self) -> String {
        let mut out = String::new();
        for r in &self.resources {
            if let Some(name) = &r.name {
                let name = name.to_ascii_uppercase();
                let id = if self.named {
                    format!("0x{:x}", r.id)
                } else {
                    r.id.to_string()
                };
                out.push_str(&format!("#define {name:<41} {id}\r\n"));
            }
        }
        out
    }
}
