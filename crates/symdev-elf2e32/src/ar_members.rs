//! `ArMembers`: the members of a System V / GNU `ar` archive, as `ar` and RVCT's `armar`
//! write them (the SDK's `.lib` files, GCCE's `libsupc++.a`, symdev's `libsymrs.a`).
use symdev_core::{Error, Result};

use crate::ar_member::ArMember;

/// Every member of an archive, in order, each checked to lie inside it.
pub(crate) struct ArMembers {
    pub members: Vec<ArMember>,
}

impl ArMembers {
    const MAGIC: &'static [u8] = b"!<arch>\n";
    const HEADER: usize = 60;

    pub fn read(bytes: &[u8]) -> Result<Self> {
        if !bytes.starts_with(Self::MAGIC) {
            return Err(Error::Other(
                "not an ar archive (no `!<arch>` magic)".into(),
            ));
        }
        let mut members: Vec<ArMember> = Vec::new();
        let mut long_names: Option<&[u8]> = None;
        let mut at = Self::MAGIC.len();
        while at < bytes.len() {
            let header = at
                .checked_add(Self::HEADER)
                .and_then(|end| bytes.get(at..end))
                .ok_or_else(|| {
                    Error::Other(format!(
                        "member header at {at} runs past the end of the archive"
                    ))
                })?;
            let field = String::from_utf8_lossy(&header[..16])
                .trim_end()
                .to_string();
            let name = Self::long_name(&field, long_names).unwrap_or(field);
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
            let end = start.checked_add(size).filter(|end| *end <= bytes.len());
            let Some(end) = end else {
                return Err(Error::Other(format!(
                    "member `{name}` at {at}: contents past the end of the archive"
                )));
            };
            if name == "//" {
                long_names = Some(&bytes[start..end]);
            }
            members.push(ArMember {
                name,
                data: start..end,
            });
            at = end.saturating_add(size & 1);
        }
        Ok(Self { members })
    }

    /// The name a `/<offset>` field points to in the `//` table: up to its `/\n`.
    fn long_name(field: &str, table: Option<&[u8]>) -> Option<String> {
        let offset: usize = field.strip_prefix('/')?.parse().ok()?;
        let rest = table?.get(offset..)?;
        let end = rest.windows(2).position(|w| w == b"/\n")?;
        Some(String::from_utf8_lossy(&rest[..end]).into_owned())
    }
}
