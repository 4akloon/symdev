use super::array::SisArray;
use super::compressed::SisCompressed;
use super::field::SisEncode;

pub struct SisData32 {
    pub compressed: SisCompressed,
}

impl SisData32 {
    pub const KIND: u32 = 32;

    pub fn new(compressed: SisCompressed) -> Self {
        Self { compressed }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.compressed.field().bytes()
    }
}

impl SisEncode for SisData32 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisData32::payload(self)
    }
}

pub struct SisData31 {
    pub items: SisArray,
}

impl SisData31 {
    pub const KIND: u32 = 31;

    pub fn new(items: SisArray) -> Self {
        Self { items }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.items.field().bytes()
    }
}

impl SisEncode for SisData31 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisData31::payload(self)
    }
}

pub struct SisData {
    pub items: SisArray,
}

impl SisData {
    pub const KIND: u32 = 30;

    pub fn new(items: SisArray) -> Self {
        Self { items }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.items.field().bytes()
    }
}

impl SisEncode for SisData {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisData::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::{SisArray, SisCompressed};

    const HELLO_DATA_SHA1: [u8; 20] = [
        0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5, 0x65,
        0xcd, 0x64, 0xea, 0x39, 0x70,
    ];

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_type30_golden() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_type30.hex"))
    }

    fn hello_exe_bytes() -> Vec<u8> {
        hello_type30_golden()[60..].to_vec()
    }

    fn hello_compressed() -> SisCompressed {
        SisCompressed {
            algorithm: 0,
            uncompressed_size: 3588,
            reserved: 0,
            data: hello_exe_bytes(),
        }
    }

    fn hello_data() -> SisData {
        SisData::new(SisArray::new(vec![
            SisData31::new(SisArray::new(vec![
                SisData32::new(hello_compressed()).field(),
            ]))
            .field(),
        ]))
    }

    #[test]
    fn hello_data32_header_matches_experiment_33() {
        let f = SisData32::new(hello_compressed()).field();
        assert_eq!(&f.bytes()[..8], &[0x20, 0, 0, 0, 0x18, 0x0e, 0, 0]);
        assert_eq!(f.payload.len(), 3608);
        assert_eq!(SisData32::KIND, 32);
    }

    #[test]
    fn hello_data_payload_hash_matches_experiment_33() {
        let data = hello_exe_bytes();
        assert_eq!(data.len(), 3588);
        assert_eq!(
            &data[..16],
            &[
                0x7a, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0xb0, 0x08,
                0x32, 0xc1
            ]
        );
        assert_eq!(
            &data[data.len() - 16..],
            &[
                0xdc, 0x95, 0x8a, 0x46, 0xa2, 0x45, 0xc4, 0x8c, 0x39, 0x38, 0x35, 0xbb, 0x91, 0x10,
                0x7f, 0xff
            ]
        );
        assert_eq!(
            HELLO_DATA_SHA1,
            [
                0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5,
                0x65, 0xcd, 0x64, 0xea, 0x39, 0x70
            ]
        );
    }

    #[test]
    fn hello_data_field_matches_experiment_33() {
        let golden = hello_type30_golden();
        assert_eq!(&golden[..8], &[0x1e, 0, 0, 0, 0x38, 0x0e, 0, 0]);
        assert_eq!(golden.len(), 3648);
        assert_eq!(hello_data().payload().len(), 3640);
        assert_eq!(hello_data().field().bytes(), golden);
        assert_eq!(SisData::KIND, 30);
        assert_eq!(SisData31::KIND, 31);
    }
}
