use symdev_uidcrc::UidCrc;

pub struct SisUid {
    pub package: u32,
}

impl SisUid {
    pub const UID1: u32 = 0x1020_1a7a;
    pub const UID2: u32 = 0;

    pub fn new(package: u32) -> Self {
        Self { package }
    }

    pub fn crc(&self) -> UidCrc {
        UidCrc::new(Self::UID1, Self::UID2, self.package)
    }

    pub fn bytes(&self) -> [u8; 16] {
        self.crc().bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_sis_uid_bytes_match_experiment_14() {
        let want = [
            0x7a, 0x1a, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0x04, 0x00,
            0xb4, 0x5d,
        ];
        assert_eq!(SisUid::new(0xe79e_4cf9).bytes(), want);
    }

    #[test]
    fn hello_sis_uid_checked_matches_experiment_14() {
        assert_eq!(SisUid::new(0xe79e_4cf9).crc().checked(), 0x5db4_0004);
    }

    #[test]
    fn sis_uid_constants_are_recorded() {
        assert_eq!(SisUid::UID1, 0x1020_1a7a);
        assert_eq!(SisUid::UID2, 0);
    }
}
