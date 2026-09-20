//! `io::Write`, with `std`'s signatures and `std`'s documented rules.
use alloc::vec::Vec;
use core::fmt;

use super::{Error, ErrorKind, Result};

/// The `std::io::Write` trait.
pub trait Write {
    /// Writes some of `buf` and returns how many bytes went out.
    ///
    /// As in `std`: a short write is not an error, and `Ok(0)` means the destination
    /// can take no more. [`ErrorKind::Interrupted`] means the call should be retried.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Pushes everything buffered on to its destination.
    fn flush(&mut self) -> Result<()>;

    /// Writes all of `buf`.
    ///
    /// As in `std`: it loops over short writes, retries on
    /// [`ErrorKind::Interrupted`], and turns a write that accepted nothing into
    /// [`ErrorKind::WriteZero`]. On error there is no telling how much was written.
    fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => return Err(Error::from(ErrorKind::WriteZero)),
                Ok(n) => buf = &buf[n..],
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Writes a formatted string, so `write!(file, "{x}")` works as it does in `std`.
    ///
    /// `core::fmt` cannot carry an error out of a formatting run, so as in `std` the
    /// original I/O error is kept aside and returned instead of `fmt::Error`.
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<()> {
        let mut adapter = Adapter {
            inner: self,
            error: Ok(()),
        };
        match fmt::write(&mut adapter, args) {
            Ok(()) => Ok(()),
            // A `fmt::Error` with nothing recorded means the `Display` impl itself
            // failed, which `std` reports as `Other`.
            Err(_) => match adapter.error {
                Err(e) => Err(e),
                Ok(()) => Err(Error::from(ErrorKind::Other)),
            },
        }
    }
}

/// Carries an [`Error`] out of a `core::fmt` run, which can only report `fmt::Error`.
struct Adapter<'a, W: ?Sized> {
    inner: &'a mut W,
    error: Result<()>,
}

impl<W: Write + ?Sized> fmt::Write for Adapter<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self.inner.write_all(s.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.error = Err(e);
                Err(fmt::Error)
            }
        }
    }
}

impl<W: Write + ?Sized> Write for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        (**self).write(buf)
    }

    fn flush(&mut self) -> Result<()> {
        (**self).flush()
    }
}

/// Writing to a `Vec<u8>` appends, as `std` does, and never fails short of the
/// allocator giving up.
impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}
