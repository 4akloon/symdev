//! `E32Uid`: the UID prefix of an E32 image.
use symdev_uidcrc::UidCrc;

/// UID prefix of an E32 image (`iUid1`/`iUid2`/`iUid3` + uidcrc checksum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E32Uid {
    pub uid1: u32,
    pub uid2: u32,
    pub uid3: u32,
}

impl E32Uid {
    /// Observed on experiment-6 `hello.exe` (`--targettype=EXE`, `--uid2` omitted).
    pub const EXE_UID2: u32 = 0;

    pub fn for_exe(uid1: u32, uid3: u32) -> Self {
        Self {
            uid1,
            uid2: Self::EXE_UID2,
            uid3,
        }
    }

    pub fn crc(&self) -> UidCrc {
        UidCrc::new(self.uid1, self.uid2, self.uid3)
    }

    pub fn bytes(&self) -> [u8; 16] {
        self.crc().bytes()
    }
}
