use super::checksum::{SisChecksum34, SisChecksum35};
use super::compressed::SisCompressed;
use super::data::SisData;
use super::field::{SisEncode, SisField};
use super::uid::SisUid;

pub struct SisUnsigned {
    pub uid: SisUid,
    pub compressed: SisCompressed,
    pub data: SisData,
}

impl SisUnsigned {
    pub const KIND: u32 = SisField::CONTROLLER;

    pub fn new(uid: SisUid, compressed: SisCompressed, data: SisData) -> Self {
        Self {
            uid,
            compressed,
            data,
        }
    }

    pub fn payload(&self) -> Vec<u8> {
        [
            SisChecksum34::of(&self.compressed.field()).field().bytes(),
            SisChecksum35::of(&self.data.field()).field().bytes(),
            self.compressed.field().bytes(),
            self.data.field().bytes(),
        ]
        .concat()
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = self.uid.bytes().to_vec();
        out.extend_from_slice(&self.field().bytes());
        out
    }
}

impl SisEncode for SisUnsigned {
    const KIND: u32 = SisUnsigned::KIND;

    fn payload(&self) -> Vec<u8> {
        SisUnsigned::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::sis::{
        SisArray, SisChecksum34, SisChecksum35, SisCompressed, SisController, SisData, SisData31,
        SisData32, SisDate, SisDateTime, SisField, SisFile, SisFiles, SisHash, SisInfo,
        SisLanguage, SisLanguages, SisPkgUid, SisProduct, SisProductVersion, SisProducts,
        SisString, SisTime, SisU32, SisUid, SisVersion, SisWord41, SisWords, SisWords16,
        SisWords19,
    };

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_sis_golden() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_sis.hex"))
    }

    fn hello_type30_golden() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_type30.hex"))
    }

    fn hello_controller() -> SisController {
        SisController::new(
            SisInfo::new(
                SisPkgUid::new(0xe79e_4cf9),
                SisString::new("Vendor"),
                SisArray::new(vec![SisString::new("hello").field()]),
                SisArray::new(vec![SisString::new("Vendor-EN").field()]),
                SisVersion::new(1, 0, 24),
                SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
            ),
            SisWords16::new(SisWords::new(vec![0x21])),
            SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
            SisProducts::new(SisArray::new(vec![
                SisProduct::new(
                    SisPkgUid::new(0x1027_52ae),
                    SisProductVersion::new(SisVersion::new(0, 0, 0)),
                    SisArray::new(vec![SisString::new("S60ProductID").field()]),
                )
                .field(),
            ])),
            SisWords19::new(SisWords::new(vec![0x14])),
            SisFiles::new(
                SisArray::new(vec![
                    SisFile::new(
                        SisString::new("!:\\sys\\bin\\hello.exe"),
                        SisString::new(""),
                        SisWord41::new(0x000b_e000),
                        SisHash::new(
                            [1, 0x25, 0x14],
                            [
                                0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b,
                                0x2a, 0x77, 0xe5, 0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
                            ],
                        ),
                        SisString::new(""),
                        [3588, 0, 3588, 0, 0],
                    )
                    .field(),
                ]),
                SisWords::new(vec![0x0d]),
                SisWords::new(vec![0x1a]),
            ),
            SisU32::new(0),
        )
    }

    fn hello_data() -> SisData {
        let exe = hello_type30_golden()[60..].to_vec();
        SisData::new(SisArray::new(vec![
            SisData31::new(SisArray::new(vec![
                SisData32::new(SisCompressed {
                    algorithm: 0,
                    uncompressed_size: 3588,
                    reserved: 0,
                    data: exe,
                })
                .field(),
            ]))
            .field(),
        ]))
    }

    fn hello_unsigned() -> SisUnsigned {
        SisUnsigned::new(
            SisUid::new(0xe79e_4cf9),
            SisCompressed::zlib(&hello_controller().field().bytes()),
            hello_data(),
        )
    }

    #[test]
    fn hello_unsigned_bytes_match_experiment_34() {
        let golden = hello_sis_golden();
        assert_eq!(golden.len(), 4000);
        assert_eq!(&golden[..16], &SisUid::new(0xe79e_4cf9).bytes());
        assert_eq!(&golden[16..24], &[0x0c, 0, 0, 0, 0x88, 0x0f, 0, 0]);
        let got = hello_unsigned().bytes();
        assert_eq!(got, golden);
        assert_eq!(SisUnsigned::KIND, SisField::CONTROLLER);
        assert_eq!(SisUnsigned::KIND, 12);
    }

    #[test]
    fn hello_unsigned_checksums_are_live_of_children() {
        let u = hello_unsigned();
        let p = u.payload();
        assert_eq!(
            &p[..12],
            &SisChecksum34::of(&u.compressed.field()).field().bytes()
        );
        assert_eq!(
            &p[12..24],
            &SisChecksum35::of(&u.data.field()).field().bytes()
        );
        assert_eq!(SisChecksum34::of(&u.compressed.field()).value, [0x5c, 0x9e]);
        assert_eq!(SisChecksum35::of(&u.data.field()).value, [0x64, 0x03]);
    }
}
