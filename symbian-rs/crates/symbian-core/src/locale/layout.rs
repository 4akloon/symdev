//! The compiled strings file, as symdev's own `rcomp` writes it — and nothing else.
//!
//! The writer is `RscCompiled::rsc_bytes` (`crates/symdev-rcomp/src/rsc.rs`); the source
//! it compiles is `StringsResources::rss` (`crates/symdev-build/src/strings_resources.rs`).
//! What that pair produces, and so all this reads:
//!
//! | offset | bytes | what |
//! |---|---|---|
//! | 0 | 16 | UID1 `0x101f4a6b`, UID2, UID3, their CRC |
//! | 16 | 1 | flags: `0x01`, "UID3 is the `NAME`" (the source has `NAME STRS`, no `UID3`) |
//! | 17 | 2 | the largest resource's size, little-endian |
//! | 19 | ⌈n/8⌉ | one bit per resource, set when it is stored packed |
//! | … | | the resources, back to back |
//! | end − 2(n+1) | 2(n+1) | `u16` offsets: entry *k* starts resource *k*+1, entry *n* is the index's own offset |
//!
//! So the file's last two bytes locate the index and give *n*. Resource 1 is the
//! signature; resource 2 + *i* is key *i*, a `BUF8` whose bytes are the value's UTF-8.
//! A `BUF8` is never compressed (`rcomp-spec.md` §3.1) and a resource is marked packed
//! only when a compressible 16-bit text is left in it (§1.4), so every bit must be clear.
//!
//! Anything else — another UID1, another flag (dictionary compression is `0x02` and up),
//! a packed resource, an index that points outside the file or backwards — is
//! [`Unreadable`], naming what was found. It is not a general `.rsc` reader and must not
//! become one by guessing.
//!
//! This file uses `core` alone: the host tests in `crates/symdev-build/tests/` include it
//! with `#[path]` and read files the real compiler wrote.

/// What in a strings file this reader does not accept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// Too short for the header and the index's last entry.
    TooShort { size: u32 },
    /// A first UID other than the one our `rcomp` writes.
    Uid1 { found: u32 },
    /// A flag byte other than `0x01`.
    Flags { found: u8 },
    /// The index offset in the last two bytes leaves no room for a whole index.
    Index { at: u32, size: u32 },
    /// A resource stored packed (compressed Unicode), which no `BUF8` string is.
    Packed { resource: u32 },
    /// A resource number the file does not have.
    NoResource { resource: u16, count: u32 },
    /// An index entry pair that runs backwards or past the index.
    Entry { resource: u16, start: u32, end: u32 },
    /// The file ended before the bytes the layout promised.
    Short { at: u32, wanted: u32 },
}

/// A file read at absolute positions.
pub trait ReadAt {
    /// The I/O error, and what an [`Unreadable`] becomes.
    type Error: From<Unreadable>;

    /// Fills all of `buf` from `at`; a file that ends first is [`Unreadable::Short`].
    fn read_exact_at(&self, at: u32, buf: &mut [u8]) -> Result<(), Self::Error>;
}

/// Where one resource's bytes are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub at: u32,
    pub len: u32,
}

/// A strings file checked once: where its index is and how many resources it has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StringsLayout {
    index_at: u32,
    count: u32,
}

impl StringsLayout {
    /// `RscCompiled::UID1`.
    pub const UID1: u32 = 0x101f_4a6b;
    /// `RscCompiled::FLAG_UID3_FROM_NAME`, the only flag the strings source produces.
    pub const FLAGS: u8 = 0x01;
    /// UIDs and CRC, flags, largest size.
    const HEADER: u32 = 19;

    /// Reads and checks the header, the index position and every packed bit of a file
    /// of `size` bytes. Three reads and one per 32 resources; nothing is kept but two
    /// numbers.
    pub fn read<R: ReadAt>(file: &R, size: u32) -> Result<Self, R::Error> {
        if size < Self::HEADER + 2 {
            return Err(Unreadable::TooShort { size }.into());
        }
        let mut head = [0u8; Self::HEADER as usize];
        file.read_exact_at(0, &mut head)?;
        let uid1 = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
        if uid1 != Self::UID1 {
            return Err(Unreadable::Uid1 { found: uid1 }.into());
        }
        if head[16] != Self::FLAGS {
            return Err(Unreadable::Flags { found: head[16] }.into());
        }
        let mut last = [0u8; 2];
        file.read_exact_at(size - 2, &mut last)?;
        let index_at = u32::from(u16::from_le_bytes(last));
        // The index is 2(n+1) bytes with n ≥ 0, runs to the end of the file, and the
        // bitmap of n bits sits between the header and the first resource.
        let tail = size.checked_sub(index_at).filter(|t| t % 2 == 0 && *t >= 2);
        let bad_index = Unreadable::Index { at: index_at, size };
        let count = tail.ok_or(bad_index)? / 2 - 1;
        if index_at < Self::HEADER + count.div_ceil(8) {
            return Err(bad_index.into());
        }
        let layout = Self { index_at, count };
        layout.check_unpacked(file)?;
        Ok(layout)
    }

    /// Every packed bit clear, read in 4-byte pieces so the stack stays small.
    fn check_unpacked<R: ReadAt>(&self, file: &R) -> Result<(), R::Error> {
        let bytes = self.count.div_ceil(8);
        let mut done = 0;
        while done < bytes {
            let mut piece = [0u8; 4];
            let n = (bytes - done).min(4);
            let piece = &mut piece[..n as usize];
            file.read_exact_at(Self::HEADER + done, piece)?;
            if let Some(i) = piece.iter().position(|b| *b != 0) {
                let bit = piece[i].trailing_zeros();
                let resource = (done + i as u32) * 8 + bit + 1;
                return Err(Unreadable::Packed { resource }.into());
            }
            done += n;
        }
        Ok(())
    }

    /// How many resources the file has.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Where resource `resource` (1-based, as resource ids count) is: one read of its
    /// two index entries.
    pub fn span<R: ReadAt>(&self, file: &R, resource: u16) -> Result<Span, R::Error> {
        if resource == 0 || u32::from(resource) > self.count {
            let count = self.count;
            return Err(Unreadable::NoResource { resource, count }.into());
        }
        let mut entries = [0u8; 4];
        let at = self.index_at + 2 * (u32::from(resource) - 1);
        file.read_exact_at(at, &mut entries)?;
        let start = u32::from(u16::from_le_bytes([entries[0], entries[1]]));
        let end = u32::from(u16::from_le_bytes([entries[2], entries[3]]));
        if start > end || end > self.index_at || start < Self::HEADER {
            return Err(Unreadable::Entry {
                resource,
                start,
                end,
            }
            .into());
        }
        Ok(Span {
            at: start,
            len: end - start,
        })
    }
}
