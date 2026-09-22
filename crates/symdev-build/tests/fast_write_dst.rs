//! The fast `write!` over every kind of destination, and the ways it must evaluate
//! like `core::write!`: the destination once, the arguments once each, in order.

// A literal argument is a case of its own: rustc folds it into the template.
#![allow(clippy::write_literal)]

#[macro_use]
mod fast_write_support;

use std::fmt::{self, Write as _};

use fast_write_support::{Custom, HostBuf};

#[test]
fn a_native_sink_appends_numbers_itself() {
    // `7u64` would be folded into the text by rustc, and so by the macro: a variable.
    let seven = 7u64;
    let mut fast = HostBuf::new(64);
    symbian_fmt::write!(fast, "{} and {}", -5, seven).unwrap();
    assert_eq!((fast.text().as_str(), fast.native_numbers), ("-5 and 7", 2));
    let mut core = HostBuf::new(64);
    core::write!(core, "{} and {}", -5, seven).unwrap();
    assert_eq!((core.text().as_str(), core.native_numbers), ("-5 and 7", 0));
    // Not on the list: through `core::fmt`, so not through `put_int`.
    let mut slow = HostBuf::new(64);
    symbian_fmt::write!(slow, "{}", Custom(4)).unwrap();
    assert_eq!((slow.text().as_str(), slow.native_numbers), ("<4>", 0));
}

/// A `Display` that uses the fast `write!` on its `Formatter`, against the same with
/// `core::write!`, rendered with and without outer flags (which a nested `write!`
/// ignores, in both).
struct Nested<const FAST: bool>(i32, &'static str);

impl<const FAST: bool> fmt::Display for Nested<FAST> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if FAST {
            symbian_fmt::write!(f, "{}:{}", self.0, self.1)
        } else {
            core::write!(f, "{}:{}", self.0, self.1)
        }
    }
}

#[test]
fn a_formatter_is_a_destination_like_any_other() {
    for v in [0, -7, i32::MIN] {
        let (fast, core) = (Nested::<true>(v, "x"), Nested::<false>(v, "x"));
        assert_eq!(
            format!("{fast}|{fast:>20}|{fast:?}", fast = fast.to_string()),
            format!("{core}|{core:>20}|{core:?}", core = core.to_string())
        );
        assert_eq!(format!("[{fast:>20}]"), format!("[{core:>20}]"));
    }
}

fn generic_fast<W: fmt::Write + ?Sized>(w: &mut W, n: u32) -> fmt::Result {
    symbian_fmt::write!(w, "n={n};")?;
    symbian_fmt::writeln!(w, "{}", Custom(n as i32))
}

fn generic_core<W: fmt::Write + ?Sized>(w: &mut W, n: u32) -> fmt::Result {
    core::write!(w, "n={n};")?;
    core::writeln!(w, "{}", Custom(n as i32))
}

#[test]
fn a_generic_destination_takes_the_fmt_write_path() {
    let (mut fast, mut core) = (String::new(), String::new());
    generic_fast(&mut fast, 12).unwrap();
    generic_core(&mut core, 12).unwrap();
    let dyn_fast: &mut dyn fmt::Write = &mut fast;
    generic_fast(dyn_fast, 3).unwrap();
    generic_core(&mut core, 3).unwrap();
    assert_eq!(fast, core);
    // A native sink seen only through its `fmt::Write` bound is still exact.
    let mut buf = HostBuf::new(64);
    generic_fast(&mut buf, 5).unwrap();
    assert_eq!((buf.text().as_str(), buf.native_numbers), ("n=5;<5>\n", 0));
}

/// An `io::Write` that takes `limit` bytes and then fails, to compare errors too.
struct Io {
    bytes: Vec<u8>,
    limit: usize,
}

impl std::io::Write for Io {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let room = self.limit - self.bytes.len();
        if room == 0 {
            return Err(std::io::Error::other("full"));
        }
        let n = room.min(buf.len());
        self.bytes.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn an_io_write_destination_formats_through_its_own_write_fmt() {
    use std::io::Write as _;
    for limit in 0..24 {
        let mut fast = Io {
            bytes: Vec::new(),
            limit,
        };
        let fast_result = symbian_fmt::write!(fast, "id={} name={} {}", 42, "abc", Custom(-1));
        let mut core = Io {
            bytes: Vec::new(),
            limit,
        };
        let core_result = core::write!(core, "id={} name={} {}", 42, "abc", Custom(-1));
        assert_eq!(fast.bytes, core.bytes, "limit {limit}");
        assert_eq!(
            fast_result.map_err(|e| e.to_string()),
            core_result.map_err(|e| e.to_string())
        );
    }
    let mut v: Vec<u8> = Vec::new();
    symbian_fmt::writeln!(v, "{}-{}", 1, 'z').unwrap();
    assert_eq!(v, b"1-z\n");
}

struct Targets {
    text: String,
    picks: u32,
}

impl Targets {
    fn pick(&mut self, log: &std::cell::RefCell<String>) -> &mut String {
        self.picks += 1;
        log.borrow_mut().push('d');
        &mut self.text
    }
}

#[test]
fn destination_and_arguments_are_evaluated_once_in_order() {
    let log = std::cell::RefCell::new(String::new());
    let step = |tag: &str, value: u32| {
        log.borrow_mut().push_str(tag);
        value
    };
    let mut fast = Targets {
        text: String::new(),
        picks: 0,
    };
    symbian_fmt::write!(fast.pick(&log), "{1}{0}{1}", step("a", 1), step("b", 2)).unwrap();
    let fast_log = log.replace(String::new());
    let mut core = Targets {
        text: String::new(),
        picks: 0,
    };
    core::write!(core.pick(&log), "{1}{0}{1}", step("a", 1), step("b", 2)).unwrap();
    let core_log = log.replace(String::new());
    assert_eq!((fast.picks, fast_log.as_str()), (1, "dab"));
    assert_eq!((fast.text, fast_log), (core.text, core_log));
}

#[test]
fn an_argument_may_read_the_destination_it_is_written_into() {
    let mut s = String::from("abc");
    symbian_fmt::write!(s, " len={}", s.len()).unwrap();
    let mut buf = HostBuf::new(32);
    symbian_fmt::write!(buf, "{}", buf.units.len()).unwrap();
    assert_eq!((s.as_str(), buf.text().as_str()), ("abc len=3", "0"));
}

#[test]
fn a_field_a_reference_and_a_reborrowed_formatter_are_destinations() {
    struct Holder {
        line: String,
    }
    let mut h = Holder {
        line: String::new(),
    };
    symbian_fmt::write!(h.line, "{}", 1).unwrap();
    symbian_fmt::write!(&mut h.line, "{}", 2).unwrap();
    let r = &mut h.line;
    symbian_fmt::write!(r, "{}", 3).unwrap();
    assert_eq!(h.line, "123");
}
