//! Destinations that record every call `write!` makes, so the fast `write!` can be held
//! to `core::write!` call for call — the same `write_str`/`write_char` calls, with the
//! same text, in the same order, and the same result. Identical calls mean identical
//! bytes in any destination, so each is also made to fail in every way that matters:
//! at the n-th call, and when a write does not fit (atomically, as `Buf16` does).

#![allow(dead_code)]

use std::fmt;

/// One call a destination received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    Str(String),
    Char(char),
}

/// How a [`Rec`] fails.
#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Never,
    /// The call with this index (0-based) and every later one fail.
    AtCall(usize),
    /// A write that would take the total past this many bytes fails and writes nothing.
    Capacity(usize),
}

/// The modes every case runs under.
pub fn modes() -> Vec<Mode> {
    let mut all = vec![Mode::Never];
    all.extend((0..8).map(Mode::AtCall));
    all.extend((0..48).map(Mode::Capacity));
    all
}

/// A `fmt::Write` that records its calls and fails as its [`Mode`] says.
#[derive(Debug)]
pub struct Rec {
    pub calls: Vec<Call>,
    pub text: String,
    mode: Mode,
    count: usize,
}

impl Rec {
    pub fn new(mode: Mode) -> Self {
        Self { calls: Vec::new(), text: String::new(), mode, count: 0 }
    }

    fn accept(&mut self, call: Call, s: &str) -> fmt::Result {
        let index = self.count;
        self.count += 1;
        self.calls.push(call);
        let ok = match self.mode {
            Mode::Never => true,
            Mode::AtCall(n) => index < n,
            Mode::Capacity(cap) => self.text.len() + s.len() <= cap,
        };
        if !ok {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

impl fmt::Write for Rec {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.accept(Call::Str(s.to_owned()), s)
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.accept(Call::Char(c), c.encode_utf8(&mut [0; 4]))
    }
}

/// A destination with a native [`symbian_fmt::Sink`], modelled on `Buf16`: a fixed
/// capacity, `write_str` all-or-nothing, and numbers appended whole by a routine of
/// its own (standing in for euser's `AppendNum`). It exists to prove that the `Sink`
/// path is taken and that its contract — the lone `-` included — is what `core` does.
#[derive(Debug)]
pub struct HostBuf {
    pub text: String,
    cap: usize,
    pub native_numbers: usize,
}

impl HostBuf {
    pub fn new(cap: usize) -> Self {
        Self { text: String::new(), cap, native_numbers: 0 }
    }

    fn push(&mut self, s: &str) -> fmt::Result {
        if self.text.len() + s.len() > self.cap {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

impl fmt::Write for HostBuf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push(s)
    }
}

impl symbian_fmt::Sink for HostBuf {
    fn put_str(&mut self, s: &str) -> fmt::Result {
        self.push(s)
    }

    fn put_char(&mut self, c: char) -> fmt::Result {
        self.push(c.encode_utf8(&mut [0; 4]))
    }

    fn put_int(&mut self, value: i64) -> fmt::Result {
        self.native_numbers += 1;
        if self.push(&value.to_string()).is_ok() {
            return Ok(());
        }
        if value < 0 {
            self.push("-")?;
        }
        Err(fmt::Error)
    }

    fn put_large(&mut self, value: u64) -> fmt::Result {
        self.native_numbers += 1;
        self.push(&value.to_string())
    }
}

/// A user type with its own `Display`, which the fast path must not touch.
pub struct Custom(pub i32);

impl fmt::Display for Custom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        fmt::Display::fmt(&self.0, f)?;
        f.write_char('>')
    }
}

use fmt::Write as _;

/// Runs the same `write!` arguments through `core::write!` and `symbian_fmt::write!`
/// into a [`Rec`] under every [`Mode`], and into a [`HostBuf`] of every capacity, and
/// asserts they did the same thing.
#[macro_export]
macro_rules! same {
    ($($t:tt)*) => {{
        use ::std::fmt::Write as _;
        for mode in $crate::fast_write_support::modes() {
            let mut core_rec = $crate::fast_write_support::Rec::new(mode);
            let core_result = ::core::write!(core_rec, $($t)*);
            let mut fast_rec = $crate::fast_write_support::Rec::new(mode);
            let fast_result = ::symbian_fmt::write!(fast_rec, $($t)*);
            assert_eq!(
                (&fast_rec.calls, fast_result),
                (&core_rec.calls, core_result),
                "write!({}) under {:?}", stringify!($($t)*), mode
            );
        }
        for cap in 0..48 {
            let mut core_buf = $crate::fast_write_support::HostBuf::new(cap);
            let core_result = ::core::write!(core_buf, $($t)*);
            let mut fast_buf = $crate::fast_write_support::HostBuf::new(cap);
            let fast_result = ::symbian_fmt::write!(fast_buf, $($t)*);
            assert_eq!(
                (&fast_buf.text, fast_result),
                (&core_buf.text, core_result),
                "write!({}) into a HostBuf of {cap}", stringify!($($t)*)
            );
        }
        let mut core_text = String::new();
        let _ = ::core::write!(core_text, $($t)*);
        core_text
    }};
}

/// As [`same!`], for `writeln!`; `same_ln!()` is `writeln!(w)`.
#[macro_export]
macro_rules! same_ln {
    (@run $c:ident, $f:ident; $($t:tt)*) => {{
        use ::std::fmt::Write as _;
        for mode in $crate::fast_write_support::modes() {
            let mut $c = $crate::fast_write_support::Rec::new(mode);
            let core_result = ::core::writeln!($c $($t)*);
            let mut $f = $crate::fast_write_support::Rec::new(mode);
            let fast_result = ::symbian_fmt::writeln!($f $($t)*);
            assert_eq!(
                (&$f.calls, fast_result),
                (&$c.calls, core_result),
                "writeln!({}) under {:?}", stringify!($($t)*), mode
            );
        }
    }};
    () => { $crate::same_ln!(@run core_rec, fast_rec; ) };
    ($($t:tt)+) => { $crate::same_ln!(@run core_rec, fast_rec; , $($t)+) };
}
