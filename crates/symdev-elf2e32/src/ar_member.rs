//! `ArMember`: one member of an `ar` archive.
use std::ops::Range;

/// A member's name — the long name the `//` table gives a `/<offset>` field, else the field
/// as written, trimmed — and where its contents lie in the archive.
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
