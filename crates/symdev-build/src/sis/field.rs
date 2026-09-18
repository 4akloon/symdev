pub struct SisField {
    pub kind: u32,
    pub payload: Vec<u8>,
}

impl SisField {
    pub const CONTROLLER: u32 = 12;

    pub fn new(kind: u32, payload: Vec<u8>) -> Self {
        Self { kind, payload }
    }

    pub fn header_bytes(&self) -> [u8; 8] {
        let mut header = [0u8; 8];
        header[..4].copy_from_slice(&self.kind.to_le_bytes());
        header[4..].copy_from_slice(&(self.payload.len() as u32).to_le_bytes());
        header
    }

    pub fn bytes(&self) -> Vec<u8> {
        let pad = (4 - (self.payload.len() % 4)) % 4;
        let mut out = Vec::with_capacity(8 + self.payload.len() + pad);
        out.extend_from_slice(&self.header_bytes());
        out.extend_from_slice(&self.payload);
        out.resize(out.len() + pad, 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_sis_controller_header_matches_experiment_15() {
        let f = SisField::new(SisField::CONTROLLER, vec![0; 3976]);
        assert_eq!(f.header_bytes(), [0x0c, 0, 0, 0, 0x88, 0x0f, 0, 0]);
    }

    #[test]
    fn hello_sisx_controller_header_matches_experiment_15() {
        let f = SisField::new(SisField::CONTROLLER, vec![0; 5148]);
        assert_eq!(f.header_bytes(), [0x0c, 0, 0, 0, 0x1c, 0x14, 0, 0]);
    }

    #[test]
    fn two_byte_field_pads_to_four() {
        let f = SisField::new(34, vec![0x5c, 0x9e]);
        assert_eq!(f.bytes(), [0x22, 0, 0, 0, 2, 0, 0, 0, 0x5c, 0x9e, 0, 0]);
    }
}
