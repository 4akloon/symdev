//! An integer's decimal digits, without a division routine.
//!
//! On this ARMv5TE target a 64-bit division is `compiler_builtins`' `__aeabi_uldivmod`
//! (`u64_div_rem`, 636 bytes), which is part of what a fast `write!` exists to leave
//! out. A 32-bit division by the constant 10 compiles to a multiply, so a 64-bit value
//! is divided in 16-bit limbs, each step dividing a number below `10 << 16` — a 32-bit
//! value — by 10.

/// A `u64`'s decimal digits: up to 20, `u64::MAX`'s count.
pub struct Decimal {
    digits: [u8; 20],
    start: usize,
}

impl Decimal {
    pub fn of(mut value: u64) -> Self {
        let mut digits = [b'0'; 20];
        let mut start = digits.len();
        loop {
            let (quotient, digit) = div10(value);
            start -= 1;
            digits[start] = b'0' + digit;
            value = quotient;
            if value == 0 {
                return Self { digits, start };
            }
        }
    }

    pub fn as_str(&self) -> &str {
        let digits = &self.digits[self.start..];
        // Every byte is an ASCII digit, so this never takes the `Err` branch.
        core::str::from_utf8(digits).unwrap_or("")
    }
}

/// `(value / 10, value % 10)`.
fn div10(value: u64) -> (u64, u8) {
    if let Ok(small) = u32::try_from(value) {
        return (u64::from(small / 10), (small % 10) as u8);
    }
    let mut quotient = 0u64;
    let mut rest = 0u32;
    for shift in [48u32, 32, 16, 0] {
        let current = (rest << 16) | ((value >> shift) as u32 & 0xffff);
        quotient = (quotient << 16) | u64::from(current / 10);
        rest = current % 10;
    }
    (quotient, rest as u8)
}
