use super::field::SisEncode;
use symdev_core::{Error, Result};

pub struct SisCompressed {
    pub algorithm: u32,
    pub uncompressed_size: u32,
    pub reserved: u32,
    pub data: Vec<u8>,
}

impl SisCompressed {
    pub const KIND: u32 = 3;
    pub const STORED: u32 = 0;
    pub const DEFLATE: u32 = 1;

    pub fn new(uncompressed_size: u32, data: Vec<u8>) -> Self {
        Self {
            algorithm: Self::DEFLATE,
            uncompressed_size,
            reserved: 0,
            data,
        }
    }

    pub fn zlib(uncompressed: &[u8]) -> Result<Self> {
        use std::io::Write;

        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(6));
        encoder
            .write_all(uncompressed)
            .map_err(|e| Error::Other(format!("zlib encode: {e}")))?;
        Ok(Self::new(
            uncompressed.len() as u32,
            encoder
                .finish()
                .map_err(|e| Error::Other(format!("zlib finish: {e}")))?,
        ))
    }

    /// makesis file blob: zlib when that is smaller, else stored (experiment 43).
    pub fn smallest(uncompressed: &[u8]) -> Result<Self> {
        let deflated = Self::zlib(uncompressed)?;
        if deflated.data.len() < uncompressed.len() {
            return Ok(deflated);
        }
        Ok(Self {
            algorithm: Self::STORED,
            uncompressed_size: uncompressed.len() as u32,
            reserved: 0,
            data: uncompressed.to_vec(),
        })
    }

    pub fn header_bytes(&self) -> [u8; 12] {
        let mut header = [0u8; 12];
        header[..4].copy_from_slice(&self.algorithm.to_le_bytes());
        header[4..8].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        header[8..].copy_from_slice(&self.reserved.to_le_bytes());
        header
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + self.data.len());
        out.extend_from_slice(&self.header_bytes());
        out.extend_from_slice(&self.data);
        out
    }
}

impl SisEncode for SisCompressed {
    const KIND: u32 = SisCompressed::KIND;

    fn payload(&self) -> Vec<u8> {
        self.bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;

    #[test]
    fn hello_sis_compressed_prefix_matches_experiment_16() {
        assert_eq!(
            SisCompressed::new(548, vec![]).header_bytes(),
            [1, 0, 0, 0, 0x24, 2, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn hello_sisx_compressed_prefix_matches_experiment_16() {
        assert_eq!(
            SisCompressed::new(1868, vec![]).header_bytes(),
            [1, 0, 0, 0, 0x4c, 7, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn compressed_field_header_is_type_3_length_295() {
        let f = SisCompressed::new(548, vec![0; 283]).field();
        assert_eq!(f.header_bytes(), [3, 0, 0, 0, 0x27, 1, 0, 0]);
        assert_eq!(SisCompressed::KIND, 3);
        assert_eq!(SisCompressed::DEFLATE, 1);
    }
}
