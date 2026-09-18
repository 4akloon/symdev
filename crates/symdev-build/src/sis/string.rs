use super::field::SisEncode;

pub struct SisString {
    pub text: String,
}

impl SisString {
    pub const KIND: u32 = 1;

    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.text
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect()
    }
}

impl SisEncode for SisString {
    const KIND: u32 = SisString::KIND;

    fn payload(&self) -> Vec<u8> {
        SisString::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;

    #[test]
    fn vendor_payload_is_utf16_le_without_nul() {
        assert_eq!(
            SisString::new("Vendor").payload(),
            [0x56, 0, 0x65, 0, 0x6e, 0, 0x64, 0, 0x6f, 0, 0x72, 0]
        );
    }

    #[test]
    fn hello_field_pads_odd_utf16_length() {
        assert_eq!(
            SisString::new("hello").field().bytes(),
            [
                1, 0, 0, 0, 0x0a, 0, 0, 0, 0x68, 0, 0x65, 0, 0x6c, 0, 0x6c, 0, 0x6f, 0, 0, 0
            ]
        );
        assert_eq!(SisString::KIND, 1);
    }
}
