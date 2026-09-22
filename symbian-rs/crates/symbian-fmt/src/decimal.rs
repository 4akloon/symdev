//! An integer's decimal digits, with no division routine and no UTF-8 check.
//!
//! On this ARMv5TE target a 64-bit division is `compiler_builtins`'
//! `__aeabi_uldivmod` (`u64_div_rem`, 636 bytes), which is part of what a fast `write!`
//! exists to leave out. A 32-bit division by the constant 10 compiles to a multiply.
//! So a value that fits a `u32` — every `i32`, `u32` and anything narrower, and most
//! 64-bit values in practice — is converted with that one loop, and only a larger one
//! takes [`Decimal::of_u64`], which divides in 16-bit limbs: each step divides a number
//! below `10 << 16`, a `u32`, by 10.
//!
//! The first version ended in `core::str::from_utf8`, whose validator was 572 bytes
//! of an image for text that is ASCII digits by construction (experiment 99).

/// A number's decimal digits: up to 20, `u64::MAX`'s count.
pub struct Decimal {
    digits: [u8; 20],
    start: usize,
}

impl Decimal {
    /// The digits of a `u32`.
    pub fn of_u32(value: u32) -> Self {
        let mut out = Self { digits: [b'0'; 20], start: 20 };
        out.prepend_u32(value);
        out
    }

    /// The digits of any `u64`.
    pub fn of_u64(mut value: u64) -> Self {
        let mut out = Self { digits: [b'0'; 20], start: 20 };
        while value > u64::from(u32::MAX) {
            let mut quotient = 0u64;
            let mut rest = 0u32;
            for shift in [48u32, 32, 16, 0] {
                let current = (rest << 16) | ((value >> shift) as u32 & 0xffff);
                quotient = (quotient << 16) | u64::from(current / 10);
                rest = current % 10;
            }
            out.prepend(rest);
            value = quotient;
        }
        out.prepend_u32(value as u32);
        out
    }

    fn prepend_u32(&mut self, mut value: u32) {
        loop {
            self.prepend(value % 10);
            value /= 10;
            if value == 0 {
                return;
            }
        }
    }

    fn prepend(&mut self, digit: u32) {
        self.start -= 1;
        self.digits[self.start] = b'0' + digit as u8;
    }

    #[allow(unsafe_code)]
    pub fn as_str(&self) -> &str {
        let digits = &self.digits[self.start..];
        // SAFETY: every byte from `start` on was written as `b'0' + d` with `d < 10`,
        // an ASCII digit, and ASCII is UTF-8.
        unsafe { core::str::from_utf8_unchecked(digits) }
    }
}
