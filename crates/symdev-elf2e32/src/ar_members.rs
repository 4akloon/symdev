//! `ArMembers`: the members of a System V / GNU `ar` archive, as `ar` and RVCT's `armar`
//! write them (the SDK's `.lib` files, GCCE's `libsupc++.a`, symdev's `libsymrs.a`).
use std::ops::Range;

use symdev_core::{Error, Result};

/// One member: its raw name field (trimmed) and where its contents lie in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArMember {
    pub name: String,
    pub data: Range<usize>,
}

impl ArMember {
    /// The archive's own tables — the symbol table `/` (`/SYM64/`) and the long-name table
    /// `//` — which hold no object.
    pub fn is_table(&self) -> bool {
        matches!(self.name.as_str(), "/" | "//" | "/SYM64/")
    }
}

/// Every member of an archive, in order.
pub(crate) struct ArMembers;

impl ArMembers {
    const MAGIC: &'static [u8] = b"!<arch>\n";
    const HEADER: usize = 60;

    pub fn read(bytes: &[u8]) -> Result<Vec<ArMember>> {
        if !bytes.starts_with(Self::MAGIC) {
            return Err(Error::Other(
                "not an ar archive (no `!<arch>` magic)".into(),
            ));
        }
        let mut members = Vec::new();
        let mut at = Self::MAGIC.len();
        while at < bytes.len() {
            let header = bytes.get(at..at + Self::HEADER).ok_or_else(|| {
                Error::Other(format!(
                    "member header at {at} runs past the end of the archive"
                ))
            })?;
            let name = String::from_utf8_lossy(&header[..16])
                .trim_end()
                .to_string();
            if &header[58..60] != b"`\n" {
                return Err(Error::Other(format!(
                    "member `{name}` at {at}: bad header terminator"
                )));
            }
            let size: usize = std::str::from_utf8(&header[48..58])
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .ok_or_else(|| Error::Other(format!("member `{name}` at {at}: bad size field")))?;
            let start = at + Self::HEADER;
            let end = start + size;
            if end > bytes.len() {
                return Err(Error::Other(format!(
                    "member `{name}` at {at}: contents past the end of the archive"
                )));
            }
            members.push(ArMember {
                name,
                data: start..end,
            });
            at = end + (size & 1);
        }
        Ok(members)
    }
}
