//! UID header of a resource file: UID1 for Unicode resources, UID2, UID3 and the
//! checksum.

use symdev_uidcrc::UidCrc;

pub struct RscUid {
    pub uid2: u32,
    pub uid3: u32,
}

impl RscUid {
    pub const UID1: u32 = 0x101f_4a6b;
    pub const REGISTRATION_UID2: u32 = 0x101f_8021;

    pub fn new(uid2: u32, uid3: u32) -> Self {
        Self { uid2, uid3 }
    }

    pub fn registration(uid3: u32) -> Self {
        Self::new(Self::REGISTRATION_UID2, uid3)
    }

    pub fn crc(&self) -> UidCrc {
        UidCrc::new(Self::UID1, self.uid2, self.uid3)
    }

    pub fn bytes(&self) -> [u8; 16] {
        self.crc().bytes()
    }
}
