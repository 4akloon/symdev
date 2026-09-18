use super::field::SisEncode;

pub struct SisWords {
    pub values: Vec<u32>,
}

impl SisWords {
    pub const KIND: u32 = 2;

    pub fn new(values: Vec<u32>) -> Self {
        Self { values }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }
}

impl SisEncode for SisWords {
    const KIND: u32 = SisWords::KIND;

    fn payload(&self) -> Vec<u8> {
        SisWords::payload(self)
    }
}

pub struct SisWords16 {
    pub words: SisWords,
}

impl SisWords16 {
    pub const KIND: u32 = 16;

    pub fn new(words: SisWords) -> Self {
        Self { words }
    }
}

impl SisEncode for SisWords16 {
    const KIND: u32 = SisWords16::KIND;

    fn payload(&self) -> Vec<u8> {
        self.words.field().bytes()
    }
}

pub struct SisWords19 {
    pub words: SisWords,
}

impl SisWords19 {
    pub const KIND: u32 = 19;

    pub fn new(words: SisWords) -> Self {
        Self { words }
    }
}

impl SisEncode for SisWords19 {
    const KIND: u32 = SisWords19::KIND;

    fn payload(&self) -> Vec<u8> {
        self.words.field().bytes()
    }
}

pub struct SisU32 {
    pub value: u32,
}

impl SisU32 {
    pub const KIND: u32 = 40;

    pub fn new(value: u32) -> Self {
        Self { value }
    }

    pub fn payload(&self) -> [u8; 4] {
        self.value.to_le_bytes()
    }
}

impl SisEncode for SisU32 {
    const KIND: u32 = SisU32::KIND;

    fn payload(&self) -> Vec<u8> {
        SisU32::payload(self).to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;

    #[test]
    fn hello_words_payload_is_raw_u32_concat() {
        assert_eq!(SisWords::new(vec![0x21]).payload(), [0x21, 0, 0, 0]);
        assert_eq!(SisWords::KIND, 2);
    }

    #[test]
    fn hello_words16_field_matches_experiment_25() {
        let f = SisWords16::new(SisWords::new(vec![0x21])).field();
        assert_eq!(
            f.bytes(),
            [
                0x10, 0, 0, 0, 0x0c, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 0x21, 0, 0, 0
            ]
        );
        assert_eq!(SisWords16::KIND, 16);
    }

    #[test]
    fn hello_words19_field_matches_experiment_26() {
        let f = SisWords19::new(SisWords::new(vec![0x14])).field();
        assert_eq!(
            f.bytes(),
            [
                0x13, 0, 0, 0, 0x0c, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 0x14, 0, 0, 0
            ]
        );
        assert_eq!(SisWords19::KIND, 19);
    }

    #[test]
    fn hello_u32_field_matches_experiment_27() {
        let f = SisU32::new(0).field();
        assert_eq!(f.bytes(), [0x28, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(SisU32::KIND, 40);
    }
}
