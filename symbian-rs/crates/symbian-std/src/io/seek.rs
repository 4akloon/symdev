//! `io::Seek` and `io::SeekFrom`, with `std`'s signatures.
use super::Result;

/// Where a [`Seek::seek`] starts counting from, as `std::io::SeekFrom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    /// From the start of the file, forwards.
    Start(u64),
    /// From the end of the file; a negative offset goes backwards.
    End(i64),
    /// From the current position; a negative offset goes backwards.
    Current(i64),
}

/// The `std::io::Seek` trait.
pub trait Seek {
    /// Moves the position and returns where it ended up, counted from the start.
    ///
    /// As in `std`: seeking past the end is allowed and the gap reads as zeros once
    /// something is written beyond it; seeking before the start is an error.
    fn seek(&mut self, pos: SeekFrom) -> Result<u64>;

    /// Back to the start.
    fn rewind(&mut self) -> Result<()> {
        self.seek(SeekFrom::Start(0)).map(|_| ())
    }

    /// The current position, counted from the start.
    fn stream_position(&mut self) -> Result<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

impl<S: Seek + ?Sized> Seek for &mut S {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        (**self).seek(pos)
    }
}
