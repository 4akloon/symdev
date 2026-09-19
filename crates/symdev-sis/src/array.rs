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
        let Some(first) = self.items.first() else {
            return Vec::new();
        };
        // SIS array payload is element type once, then each child without repeating the type.
        let mut out = first.kind.to_le_bytes().to_vec();
        for f in &self.items {
            let full = f.bytes();
            out.extend_from_slice(&full[4..]);
        }
        out
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

    #[test]
    fn two_item_array_repeats_len_not_type() {
        let f = SisArray::new(vec![
            SisString::new("ab").field(),
            SisString::new("c").field(),
        ])
        .field();
        let b = f.bytes();
        assert_eq!(&b[..8], [2, 0, 0, 0, 20, 0, 0, 0]);
        assert_eq!(&b[8..12], [1, 0, 0, 0]); // element type once
        // two string bodies: len 4 "ab" pad0; len 2 "c\0" pad
        assert_eq!(&b[12..20], [4, 0, 0, 0, b'a', 0, b'b', 0]);
        assert_eq!(&b[20..28], [2, 0, 0, 0, b'c', 0, 0, 0]);
    }
}
