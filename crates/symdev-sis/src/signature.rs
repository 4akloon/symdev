use super::array::SisArray;
use super::field::SisEncode;
use super::string::SisString;

// type 37 is opaque DSA DER (+ 4-byte pad) or cert DER; live sign fills DSA from signed_bytes
pub struct SisBlob37 {
    pub data: Vec<u8>,
}

impl SisBlob37 {
    pub const KIND: u32 = 37;

    pub fn new(data: impl Into<Vec<u8>>) -> Self {
        Self { data: data.into() }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.data.clone()
    }
}

impl SisEncode for SisBlob37 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisBlob37::payload(self)
    }
}

pub struct SisAlgorithm38 {
    pub oid: SisString,
}

impl SisAlgorithm38 {
    pub const KIND: u32 = 38;

    pub fn new(oid: SisString) -> Self {
        Self { oid }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.oid.field().bytes()
    }
}

impl SisEncode for SisAlgorithm38 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisAlgorithm38::payload(self)
    }
}

pub struct SisSignature36 {
    pub algorithm: SisAlgorithm38,
    pub value: SisBlob37,
}

impl SisSignature36 {
    pub const KIND: u32 = 36;

    pub fn new(algorithm: SisAlgorithm38, value: SisBlob37) -> Self {
        Self { algorithm, value }
    }

    pub fn payload(&self) -> Vec<u8> {
        [self.algorithm.field().bytes(), self.value.field().bytes()].concat()
    }
}

impl SisEncode for SisSignature36 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisSignature36::payload(self)
    }
}

pub struct SisChain22 {
    pub cert: SisBlob37,
}

impl SisChain22 {
    pub const KIND: u32 = 22;

    pub fn new(cert: SisBlob37) -> Self {
        Self { cert }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.cert.field().bytes()
    }
}

impl SisEncode for SisChain22 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisChain22::payload(self)
    }
}

pub struct SisSignatures39 {
    pub signatures: SisArray,
    pub chain: SisChain22,
}

impl SisSignatures39 {
    pub const KIND: u32 = 39;

    pub fn new(signatures: SisArray, chain: SisChain22) -> Self {
        Self { signatures, chain }
    }

    pub fn payload(&self) -> Vec<u8> {
        [self.signatures.field().bytes(), self.chain.field().bytes()].concat()
    }
}

impl SisEncode for SisSignatures39 {
    const KIND: u32 = Self::KIND;

    fn payload(&self) -> Vec<u8> {
        SisSignatures39::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::{SisArray, SisString};

    const HELLO_CERT_SHA1: [u8; 20] = [
        0x66, 0x98, 0xf4, 0x84, 0xc9, 0xc6, 0x4d, 0x0d, 0xdf, 0x44, 0x24, 0x05, 0x20, 0xf0, 0xe6,
        0xbd, 0x62, 0x9a, 0xcf, 0xd9,
    ];

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_type39_golden() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_type39.hex"))
    }

    fn hello_sig_blob() -> [u8; 48] {
        [
            0x30, 0x2c, 0x02, 0x14, 0x4d, 0xe0, 0xcf, 0xec, 0x52, 0x8a, 0x05, 0x95, 0x13, 0x7d,
            0xfc, 0x0c, 0x66, 0x34, 0xe4, 0x00, 0x75, 0x28, 0xae, 0xa0, 0x02, 0x14, 0x50, 0x20,
            0x97, 0x21, 0xc3, 0x8a, 0xb4, 0xdd, 0xb9, 0xc0, 0x1d, 0x71, 0x53, 0xd3, 0x3d, 0xe7,
            0x10, 0x62, 0xa8, 0xc0, 0x00, 0x00,
        ]
    }

    fn hello_der() -> Vec<u8> {
        hello_type39_golden()[148..148 + 1171].to_vec()
    }

    fn hello_signatures() -> SisSignatures39 {
        SisSignatures39::new(
            SisArray::new(vec![
                SisSignature36::new(
                    SisAlgorithm38::new(SisString::new("1.2.840.10040.4.3")),
                    SisBlob37::new(hello_sig_blob().to_vec()),
                )
                .field(),
            ]),
            SisChain22::new(SisBlob37::new(hello_der())),
        )
    }

    #[test]
    fn hello_algorithm38_field_matches_experiment_36() {
        let f = SisAlgorithm38::new(SisString::new("1.2.840.10040.4.3")).field();
        assert_eq!(
            f.bytes(),
            [
                0x26, 0, 0, 0, 0x2c, 0, 0, 0, 1, 0, 0, 0, 0x22, 0, 0, 0, 0x31, 0, 0x2e, 0, 0x32, 0,
                0x2e, 0, 0x38, 0, 0x34, 0, 0x30, 0, 0x2e, 0, 0x31, 0, 0x30, 0, 0x30, 0, 0x34, 0,
                0x30, 0, 0x2e, 0, 0x34, 0, 0x2e, 0, 0x33, 0, 0, 0,
            ]
        );
        assert_eq!(SisAlgorithm38::KIND, 38);
    }

    #[test]
    fn hello_signature36_field_matches_experiment_36() {
        let f = SisSignature36::new(
            SisAlgorithm38::new(SisString::new("1.2.840.10040.4.3")),
            SisBlob37::new(hello_sig_blob().to_vec()),
        )
        .field();
        assert_eq!(f.bytes().len(), 116);
        assert_eq!(&f.bytes()[..8], &[0x24, 0, 0, 0, 0x6c, 0, 0, 0]);
        assert_eq!(&f.bytes()[68..], hello_sig_blob());
        assert_eq!(SisSignature36::KIND, 36);
        assert_eq!(SisBlob37::KIND, 37);
    }

    #[test]
    fn hello_signatures39_field_matches_experiment_36() {
        let golden = hello_type39_golden();
        let der = hello_der();
        assert_eq!(&golden[..8], &[0x27, 0, 0, 0, 0x20, 0x05, 0, 0]);
        assert_eq!(golden.len(), 1320);
        assert_eq!(der.len(), 1171);
        assert_eq!(
            &der[..16],
            &[
                0x30, 0x82, 0x04, 0x8f, 0x30, 0x82, 0x04, 0x4f, 0xa0, 0x03, 0x02, 0x01, 0x02, 0x02,
                0x01, 0x01
            ]
        );
        assert_eq!(
            &der[der.len() - 16..],
            &[
                0xdd, 0x66, 0xaf, 0xc8, 0xfa, 0x58, 0x67, 0x3b, 0x8f, 0xe1, 0x7c, 0x80, 0xb5, 0x61,
                0x61, 0x60
            ]
        );
        assert_eq!(
            HELLO_CERT_SHA1,
            [
                0x66, 0x98, 0xf4, 0x84, 0xc9, 0xc6, 0x4d, 0x0d, 0xdf, 0x44, 0x24, 0x05, 0x20, 0xf0,
                0xe6, 0xbd, 0x62, 0x9a, 0xcf, 0xd9
            ]
        );
        assert_eq!(hello_signatures().payload().len(), 1312);
        assert_eq!(hello_signatures().field().bytes(), golden);
        assert_eq!(SisSignatures39::KIND, 39);
        assert_eq!(SisChain22::KIND, 22);
    }
}
