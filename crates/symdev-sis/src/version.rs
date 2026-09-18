use super::field::SisEncode;

pub struct SisVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

impl SisVersion {
    pub const KIND: u32 = 4;

    pub fn new(major: u32, minor: u32, build: u32) -> Self {
        Self {
            major,
            minor,
            build,
        }
    }

    pub fn payload(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[..4].copy_from_slice(&self.major.to_le_bytes());
        out[4..8].copy_from_slice(&self.minor.to_le_bytes());
        out[8..].copy_from_slice(&self.build.to_le_bytes());
        out
    }
}

impl SisEncode for SisVersion {
    const KIND: u32 = SisVersion::KIND;

    fn payload(&self) -> Vec<u8> {
        SisVersion::payload(self).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;

    #[test]
    fn hello_pkg_version_payload_matches_experiment_18() {
        assert_eq!(
            SisVersion::new(1, 0, 24).payload(),
            [1, 0, 0, 0, 0, 0, 0, 0, 0x18, 0, 0, 0]
        );
    }

    #[test]
    fn version_field_is_type_4_length_12() {
        let f = SisVersion::new(1, 0, 24).field();
        assert_eq!(f.header_bytes(), [4, 0, 0, 0, 0x0c, 0, 0, 0]);
        assert_eq!(SisVersion::KIND, 4);
    }
}
