//! `io::Read`, with `std`'s signatures and `std`'s documented rules.
use alloc::vec::Vec;

use super::{Error, ErrorKind, Result};

/// How many bytes `read_to_end` asks for at a time. `std` grows its probe adaptively;
/// this is one fixed chunk, which is the right trade on a phone where the allocator
/// hands out 8-aligned cells with a 36-byte minimum (experiment 68).
const READ_TO_END_CHUNK: usize = 512;

/// The `std::io::Read` trait.
pub trait Read {
    /// Reads some bytes into `buf` and returns how many.
    ///
    /// As in `std`: `Ok(0)` means the source has no more bytes, a short read is not an
    /// error, and [`ErrorKind::Interrupted`] means the call should be retried.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Reads exactly `buf.len()` bytes.
    ///
    /// As in `std`: retries on [`ErrorKind::Interrupted`], and an early end of input is
    /// [`ErrorKind::UnexpectedEof`]. The contents of `buf` are unspecified on error.
    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => return Err(Error::from(ErrorKind::UnexpectedEof)),
                Ok(n) => buf = &mut buf[n..],
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Appends everything left in the source to `buf` and returns how many bytes that
    /// was.
    ///
    /// As in `std`: it reads until `read` returns `Ok(0)`, retries on
    /// [`ErrorKind::Interrupted`], and on error `buf` keeps whatever had already
    /// arrived.
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> Result<usize> {
        let start = buf.len();
        loop {
            let filled = buf.len();
            buf.resize(filled + READ_TO_END_CHUNK, 0);
            match self.read(&mut buf[filled..]) {
                Ok(0) => {
                    buf.truncate(filled);
                    return Ok(filled - start);
                }
                Ok(n) => buf.truncate(filled + n),
                Err(e) => {
                    buf.truncate(filled);
                    if e.kind() != ErrorKind::Interrupted {
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl<R: Read + ?Sized> Read for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        (**self).read(buf)
    }
}

/// Reading from a byte slice, as `std` does: the slice shortens as it is consumed and
/// an exhausted one reads `Ok(0)` for ever.
impl Read for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = buf.len().min(self.len());
        let (head, tail) = self.split_at(n);
        buf[..n].copy_from_slice(head);
        *self = tail;
        Ok(n)
    }
}
