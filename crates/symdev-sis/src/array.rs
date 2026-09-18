use super::field::{SisEncode, SisField};

pub struct SisArray {
    pub items: Vec<SisField>,
}

impl SisArray {
    pub const KIND: u32 = 2;

    pub fn new(items: Vec<SisField>) -> Self {
        Self { items }
    }

    pub fn payload(&self) -> Vec<u8> {
        self.items.iter().flat_map(|f| f.bytes()).collect()
    }
}

impl SisEncode for SisArray {
    const KIND: u32 = SisArray::KIND;

    fn payload(&self) -> Vec<u8> {
        SisArray::payload(self)
    }
}

#[cfg(test)]
mod tests {
    use super::SisEncode;
    use super::*;
    use crate::SisString;

    #[test]
    fn hello_name_array_matches_experiment_20() {
        let f = SisArray::new(vec![SisString::new("hello").field()]).field();
        assert_eq!(
            f.bytes(),
            [
                2, 0, 0, 0, 0x14, 0, 0, 0, 1, 0, 0, 0, 0x0a, 0, 0, 0, 0x68, 0, 0x65, 0, 0x6c, 0,
                0x6c, 0, 0x6f, 0, 0, 0
            ]
        );
        assert_eq!(SisArray::KIND, 2);
    }
}
