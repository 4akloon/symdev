//! `Writer`: little-endian byte-slice writer for the `.dso` layout.

pub(super) struct Writer<'a> {
    pub(super) out: &'a mut [u8],
}

impl Writer<'_> {
    pub(super) fn put(&mut self, at: usize, bytes: &[u8]) {
        self.out[at..at + bytes.len()].copy_from_slice(bytes);
    }

    pub(super) fn u16(&mut self, at: usize, v: u16) {
        self.put(at, &v.to_le_bytes());
    }

    pub(super) fn u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }
}
