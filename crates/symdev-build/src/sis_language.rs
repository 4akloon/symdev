use crate::sis_array::SisArray;
use crate::sis_field::SisField;

pub struct SisLanguage {
    pub id: u32,
}

impl SisLanguage {
    pub const KIND: u32 = 11;

    pub fn new(id: u32) -> Self {
        Self { id }
    }

    pub fn payload(&self) -> [u8; 4] {
        self.id.to_le_bytes()
    }

    pub fn field(&self) -> SisField {
        SisField::new(Self::KIND, self.payload().to_vec())
    }
}

pub struct SisLanguages {
    pub languages: SisArray,
}

impl SisLanguages {
    pub const KIND: u32 = 15;

    pub fn new(languages: SisArray) -> Self {
        Self { languages }
    }

    pub fn field(&self) -> SisField {
        SisField::new(Self::KIND, self.languages.field().bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sis_array::SisArray;

    #[test]
    fn hello_language_payload_matches_experiment_23() {
        assert_eq!(SisLanguage::new(1).payload(), [1, 0, 0, 0]);
        assert_eq!(SisLanguage::KIND, 11);
    }

    #[test]
    fn hello_languages_field_matches_experiment_23() {
        let f = SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])).field();
        assert_eq!(
            f.bytes(),
            [
                0x0f, 0, 0, 0, 0x14, 0, 0, 0, 2, 0, 0, 0, 0x0c, 0, 0, 0, 0x0b, 0, 0, 0, 4, 0, 0, 0,
                1, 0, 0, 0
            ]
        );
        assert_eq!(SisLanguages::KIND, 15);
    }
}
