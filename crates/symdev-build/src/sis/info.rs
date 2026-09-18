use super::array::SisArray;
use super::datetime::SisDateTime;
use super::field::SisEncode;
use super::pkg_uid::SisPkgUid;
use super::string::SisString;
use super::version::SisVersion;

pub struct SisInfo {
    pub uid: SisPkgUid,
    pub vendor: SisString,
    pub names: SisArray,
    pub vendor_names: SisArray,
    pub version: SisVersion,
    pub datetime: SisDateTime,
}

impl SisInfo {
    pub const KIND: u32 = 14;

    pub fn new(
        uid: SisPkgUid,
        vendor: SisString,
        names: SisArray,
        vendor_names: SisArray,
        version: SisVersion,
        datetime: SisDateTime,
    ) -> Self {
        Self {
            uid,
            vendor,
            names,
            vendor_names,
            version,
            datetime,
        }
    }

    pub fn payload(&self) -> Vec<u8> {
        let mut out = [
            self.uid.field().bytes(),
            self.vendor.field().bytes(),
            self.names.field().bytes(),
            self.vendor_names.field().bytes(),
            self.version.field().bytes(),
            self.datetime.field().bytes(),
        ]
        .concat();
        out.extend_from_slice(&[0, 0]);
        out
    }
}

impl SisEncode for SisInfo {
    const KIND: u32 = SisInfo::KIND;

    fn payload(&self) -> Vec<u8> {
        SisInfo::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::sis::{SisDate, SisTime};

    fn hello_info() -> SisInfo {
        SisInfo::new(
            SisPkgUid::new(0xe79e_4cf9),
            SisString::new("Vendor"),
            SisArray::new(vec![SisString::new("hello").field()]),
            SisArray::new(vec![SisString::new("Vendor-EN").field()]),
            SisVersion::new(1, 0, 24),
            SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
        )
    }

    #[test]
    fn hello_info_payload_is_148_children_plus_two_zeros() {
        let p = hello_info().payload();
        assert_eq!(p.len(), 150);
        assert_eq!(&p[148..], &[0, 0]);
    }

    #[test]
    fn hello_info_field_matches_experiment_22() {
        assert_eq!(
            hello_info().field().bytes(),
            [
                0x0e, 0x00, 0x00, 0x00, 0x96, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x04, 0x00,
                0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00,
                0x56, 0x00, 0x65, 0x00, 0x6e, 0x00, 0x64, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00,
                0x68, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x00, 0x00, 0x02, 0x00,
                0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00,
                0x56, 0x00, 0x65, 0x00, 0x6e, 0x00, 0x64, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x2d, 0x00,
                0x45, 0x00, 0x4e, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x08, 0x00,
                0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
                0xea, 0x07, 0x08, 0x11, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x0f, 0x12,
                0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
        assert_eq!(SisInfo::KIND, 14);
    }
}
