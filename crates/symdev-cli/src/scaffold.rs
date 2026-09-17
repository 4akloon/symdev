pub fn uid3_for_name(name: &str) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in name.as_bytes() {
        h ^= u32::from(*b);
        h = h.wrapping_mul(0x01000193);
    }
    0xE0000000 | (h & 0x0FFFFFFF)
}

pub fn uid3_hex(name: &str) -> String {
    format!("0x{:08x}", uid3_for_name(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uid3_hello_is_pinned() {
        assert_eq!(uid3_for_name("hello"), 0xef9f2cab);
        assert_eq!(uid3_hex("hello"), "0xef9f2cab");
    }
}
