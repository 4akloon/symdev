//! `HBuf16`: an `HBufC16` — one heap cell holding the header word and the code units.
use core::fmt;
use core::ptr::NonNull;

use alloc::alloc::Layout;
use symbian_sys::des::TDesC16;

use super::{DesC16, EBUFC, MAX_LENGTH, header, sealed, utf16};
use crate::{ErrorKind, Result, SymbianError};

/// The header word in front of the code units, observed at offset 0 with the data at +4.
const HEADER: usize = 4;

/// A descriptor whose text lives in one heap cell, the shape of `HBufC16`.
///
/// The cell is `[header word][code units…]`, exactly what `HBufC16::New` produces (the
/// probe read `HBufC16::New(8)`'s header as `0x00000000` — `EBufC` — and its `Ptr()` at
/// `+4`). The cell comes from the global allocator, which is `User::Alloc` on the calling
/// thread's heap, so the value is neither `Send` nor `Sync`: a `NonNull` field makes the
/// compiler enforce what the heap requires.
pub struct HBuf16 {
    cell: NonNull<u8>,
    capacity: usize,
}

impl HBuf16 {
    /// The bytes one cell of `capacity` code units needs, or `None` when the capacity is
    /// beyond what a descriptor length or a `Layout` can express.
    fn layout(capacity: usize) -> Option<Layout> {
        if capacity > MAX_LENGTH {
            return None;
        }
        Layout::from_size_align(HEADER + capacity.checked_mul(2)?, 4).ok()
    }

    /// An empty buffer with room for `capacity` code units.
    ///
    /// Fails with `KErrTooBig` for a capacity no descriptor could hold and
    /// `KErrNoMemory` when `User::Alloc` returns null.
    pub fn with_capacity(capacity: usize) -> Result<Self> {
        let layout = Self::layout(capacity).ok_or(SymbianError::of(ErrorKind::TooBig))?;
        // SAFETY: the layout has a non-zero size (the header is always there) and an
        // alignment of 4, which is below the heap's observed 8, so the global allocator
        // hands the request straight to `User::Alloc`.
        let cell = unsafe { alloc::alloc::alloc(layout) };
        let cell = NonNull::new(cell).ok_or(SymbianError::of(ErrorKind::NoMemory))?;
        // SAFETY: `cell` is a fresh, 4-aligned cell of at least `HEADER` bytes, owned
        // here and not yet shared with anything.
        unsafe { cell.cast::<u32>().write(header(EBUFC, 0)) };
        Ok(Self { cell, capacity })
    }

    /// A buffer holding `s`, sized to fit it exactly.
    pub fn from_str(s: &str) -> Result<Self> {
        let mut buf = Self::with_capacity(utf16::utf16_len(s))?;
        buf.push_str(s)?;
        Ok(buf)
    }

    /// The code units the cell can hold before it has to grow.
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// The header word, for a layout check.
    pub fn header_word(&self) -> u32 {
        // SAFETY: the cell always starts with the header word this type wrote.
        unsafe { self.cell.cast::<u32>().read() }
    }

    fn length(&self) -> usize {
        (self.header_word() & MAX_LENGTH as u32) as usize
    }

    fn set_length(&mut self, len: usize) {
        // SAFETY: `self` owns the cell exclusively here, and the header word is the
        // first four bytes of it.
        unsafe { self.cell.cast::<u32>().write(header(EBUFC, len)) };
    }

    fn units_mut(&mut self) -> &mut [u16] {
        let capacity = self.capacity;
        // SAFETY: the cell holds `HEADER + 2 * capacity` bytes; the data starts at
        // `+HEADER`, which is 4-aligned (so also 2-aligned), and `&mut self` means no
        // other reference to those units exists.
        unsafe {
            core::slice::from_raw_parts_mut(self.cell.as_ptr().add(HEADER).cast::<u16>(), capacity)
        }
    }

    /// Makes room for `extra` more code units, moving the cell if it has to.
    fn reserve(&mut self, extra: usize) -> Result<()> {
        let needed = self
            .length()
            .checked_add(extra)
            .ok_or(SymbianError::of(ErrorKind::TooBig))?;
        if needed <= self.capacity {
            return Ok(());
        }
        let wanted = needed.max(self.capacity.saturating_mul(2));
        let old = Self::layout(self.capacity).ok_or(SymbianError::of(ErrorKind::TooBig))?;
        let new = Self::layout(wanted).ok_or(SymbianError::of(ErrorKind::TooBig))?;
        // SAFETY: `old` is the layout this cell was allocated with, `new.size()` is
        // greater than zero, and the allocator's `realloc` is `User::ReAlloc` in mode 0,
        // which keeps the contents and returns null without touching the old cell on
        // failure.
        let grown = unsafe { alloc::alloc::realloc(self.cell.as_ptr(), old, new.size()) };
        self.cell = NonNull::new(grown).ok_or(SymbianError::of(ErrorKind::NoMemory))?;
        self.capacity = wanted;
        Ok(())
    }

    /// Appends `s` as UTF-16, growing the cell if it does not fit.
    pub fn push_str(&mut self, s: &str) -> Result<()> {
        let extra = utf16::utf16_len(s);
        self.reserve(extra)?;
        let len = self.length();
        utf16::encode_utf16_into(s, &mut self.units_mut()[len..len + extra])?;
        self.set_length(len + extra);
        Ok(())
    }

    /// Fills `out` with the buffer's text as UTF-8 and returns it. Nothing is allocated.
    pub fn to_str<'b>(&self, out: &'b mut [u8]) -> Result<&'b str> {
        utf16::decode_utf16_into(self.units(), out)
    }
}

impl Drop for HBuf16 {
    fn drop(&mut self) {
        if let Some(layout) = Self::layout(self.capacity) {
            // SAFETY: the cell came from the global allocator with this same layout and
            // is freed exactly once, on the thread that allocated it.
            unsafe { alloc::alloc::dealloc(self.cell.as_ptr(), layout) };
        }
    }
}

impl sealed::Sealed for HBuf16 {}

impl DesC16 for HBuf16 {
    fn as_tdesc16(&self) -> *const TDesC16 {
        self.cell.as_ptr().cast()
    }

    fn units(&self) -> &[u16] {
        let len = self.length();
        // SAFETY: the first `len` units after the header have been written by this type
        // and `&self` keeps them alive and unaliased for the borrow.
        unsafe { core::slice::from_raw_parts(self.cell.as_ptr().add(HEADER).cast::<u16>(), len) }
    }
}

impl fmt::Write for HBuf16 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s).map_err(|_| fmt::Error)
    }
}
