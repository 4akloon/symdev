use crate::sis_field::SisField;

pub struct SisPkgUid {
    pub uid: u32,
}

impl SisPkgUid {
    pub const KIND: u32 = 9;

    pub fn new(uid: u32) -> Self {
        Self { uid }
    }

    pub fn payload(&self) -> [u8; 4] {
        self.uid.to_le_bytes()
    }

    pub fn field(&self) -> SisField {
        SisField::new(Self::KIND, self.payload().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_pkg_uid_payload_matches_experiment_19() {
        assert_eq!(
            SisPkgUid::new(0xe79e_4cf9).payload(),
            [0xf9, 0x4c, 0x9e, 0xe7]
        );
    }

    #[test]
    fn pkg_uid_field_is_type_9_length_4() {
        let f = SisPkgUid::new(0xe79e_4cf9).field();
        assert_eq!(f.header_bytes(), [9, 0, 0, 0, 4, 0, 0, 0]);
        assert_eq!(SisPkgUid::KIND, 9);
    }
}
