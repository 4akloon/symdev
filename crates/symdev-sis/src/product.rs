use super::array::SisArray;
use super::field::SisEncode;
use super::pkg_uid::SisPkgUid;
use super::version::SisVersion;

pub struct SisProductVersion {
    pub version: SisVersion,
}

impl SisProductVersion {
    pub const KIND: u32 = 5;

    pub fn new(version: SisVersion) -> Self {
        Self { version }
    }
}

impl SisEncode for SisProductVersion {
    const KIND: u32 = SisProductVersion::KIND;

    fn payload(&self) -> Vec<u8> {
        self.version.field().bytes()
    }
}

pub struct SisProduct {
    pub uid: SisPkgUid,
    pub version: SisProductVersion,
    pub names: SisArray,
}

impl SisProduct {
    pub const KIND: u32 = 18;

    pub fn new(uid: SisPkgUid, version: SisProductVersion, names: SisArray) -> Self {
        Self {
            uid,
            version,
            names,
        }
    }

    pub fn payload(&self) -> Vec<u8> {
        let mut out = self.uid.field().bytes();
        out.extend(self.version.field().bytes());
        out.extend(self.names.field().bytes());
        out
    }
}

impl SisEncode for SisProduct {
    const KIND: u32 = SisProduct::KIND;

    fn payload(&self) -> Vec<u8> {
        SisProduct::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::{SisArray, SisPkgUid, SisString, SisVersion};

    fn hello_product() -> SisProduct {
        SisProduct::new(
            SisPkgUid::new(0x1027_52ae),
            SisProductVersion::new(SisVersion::new(0, 0, 0)),
            SisArray::new(vec![SisString::new("S60ProductID").field()]),
        )
    }

    #[test]
    fn hello_product_version_field_matches_experiment_24() {
        let f = SisProductVersion::new(SisVersion::new(0, 0, 0)).field();
        assert_eq!(
            f.bytes(),
            [
                0x05, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ]
        );
        assert_eq!(SisProductVersion::KIND, 5);
    }

    #[test]
    fn hello_product_field_matches_experiment_24() {
        assert_eq!(
            hello_product().field().bytes(),
            [
                0x12, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x04, 0x00,
                0x00, 0x00, 0xae, 0x52, 0x27, 0x10, 0x05, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
                0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x53, 0x00, 0x36, 0x00, 0x30, 0x00,
                0x50, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x64, 0x00, 0x75, 0x00, 0x63, 0x00, 0x74, 0x00,
                0x49, 0x00, 0x44, 0x00,
            ]
        );
        assert_eq!(SisProduct::KIND, 18);
    }
}
