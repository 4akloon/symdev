//! The phone's strings-file reader against files our own `rcomp` wrote.
//!
//! `layout.rs` is the reader `symbian_core::locale` runs on the phone, included here
//! unchanged: it uses `core` alone, so the same bytes of source are tested on the host
//! against the writer's real output (`StringsResources::rss` through the native
//! preprocessor and `Rcomp`, then `RscCompiled::rsc_bytes`).
#[path = "../../../symbian-rs/crates/symbian-core/src/locale/layout.rs"]
#[allow(dead_code)]
mod layout;

use std::collections::BTreeMap;

use layout::{ReadAt, Span, StringsLayout, Unreadable};
use symdev_build::StringsResources;
use symdev_locale::{Locales, Table};

/// A file in memory, read the way `RFile::Read(TInt, TDes8&)` reads one.
struct Bytes(Vec<u8>);

impl ReadAt for Bytes {
    type Error = Unreadable;

    fn read_exact_at(&self, at: u32, buf: &mut [u8]) -> Result<(), Unreadable> {
        let wanted = buf.len() as u32;
        let src = usize::try_from(at)
            .ok()
            .and_then(|at| self.0.get(at..at + buf.len()))
            .ok_or(Unreadable::Short { at, wanted })?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

fn compile_rss(rss: &str) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("s.rss");
    std::fs::write(&path, rss).unwrap();
    let rpp = symdev_rcomp::CPreprocessor::for_rss(&[], &[])
        .run(&path)
        .unwrap();
    symdev_rcomp::Rcomp::compile(&rpp, "s.rss")
        .unwrap()
        .rsc_bytes()
        .unwrap()
}

fn strings(pairs: &[(String, String)]) -> (StringsResources, Vec<u8>) {
    let default = Table {
        entries: pairs.iter().cloned().collect::<BTreeMap<_, _>>(),
    };
    let s = StringsResources {
        app: "demo".into(),
        locales: Locales {
            default,
            variants: Vec::new(),
        },
    };
    let file = compile_rss(&s.rss(&s.locales.default));
    (s, file)
}

fn read(file: &Bytes, resource: u16) -> Result<Vec<u8>, Unreadable> {
    let layout = StringsLayout::read(file, file.0.len() as u32)?;
    let Span { at, len } = layout.span(file, resource)?;
    let mut out = vec![0; len as usize];
    file.read_exact_at(at, &mut out)?;
    Ok(out)
}

/// Every key's value, read back through the phone's reader, is its exact UTF-8 — for
/// 70 keys (a nine-byte packed-bit map, read in three pieces), an empty value and
/// non-ASCII bytes.
#[test]
fn every_resource_reads_back_as_the_value_the_writer_was_given() {
    let mut pairs: Vec<(String, String)> = (0..64)
        .map(|i| (format!("k{i:02}"), format!("value number {i}")))
        .collect();
    pairs.extend(
        [
            ("x_empty", ""),
            ("x_latin", "d'accord é ü"),
            ("x_cyrillic", "Привіт з Rust"),
            ("x_controls", "line\nnext\ttab \"q\" \\"),
            ("x_high", "\u{9f}\u{80}\u{ff}"),
            ("x_long", &"long ".repeat(60)),
        ]
        .map(|(k, v)| (k.to_string(), v.to_string())),
    );
    let (s, bytes) = strings(&pairs);
    let file = Bytes(bytes);
    let layout = StringsLayout::read(&file, file.0.len() as u32).unwrap();
    assert_eq!(layout.count(), pairs.len() as u32 + 1, "the signature and every key");
    for (key, value) in &pairs {
        let index = s.locales.index(key).unwrap();
        assert_eq!(read(&file, index).unwrap(), value.as_bytes(), "{key} at {index}");
    }
    let signature = read(&file, 1).unwrap();
    assert_eq!(signature.len(), 8, "LONG signature, SRLINK self");
    assert_eq!(&signature[..4], &4u32.to_le_bytes());
}

#[test]
fn a_single_key_file_reads_back() {
    let (_, bytes) = strings(&[("only".into(), "x".into())]);
    assert_eq!(read(&Bytes(bytes), 2).unwrap(), b"x");
}

#[test]
fn a_resource_the_file_does_not_have_is_named() {
    let (_, bytes) = strings(&[("a".into(), "b".into())]);
    let file = Bytes(bytes);
    let none = |resource| Unreadable::NoResource { resource, count: 2 };
    assert_eq!(read(&file, 0), Err(none(0)));
    assert_eq!(read(&file, 3), Err(none(3)));
}

#[test]
fn another_uid1_is_refused() {
    let (_, mut bytes) = strings(&[("a".into(), "b".into())]);
    bytes[0] ^= 1;
    let found = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    assert_eq!(read(&Bytes(bytes), 2), Err(Unreadable::Uid1 { found }));
}

/// The writer's own output with a `UID3` statement: flag byte 0.
#[test]
fn a_file_with_its_own_uid3_is_refused_by_its_flag() {
    let bytes = compile_rss("UID3 0x12345678\nSTRUCT S { BUF8 t; }\nRESOURCE S { t = \"a\"; }\n");
    assert_eq!(bytes[16], 0);
    assert_eq!(read(&Bytes(bytes), 1), Err(Unreadable::Flags { found: 0 }));
}

/// The writer's own output with a 16-bit text it compresses: the packed bit is set.
#[test]
fn a_packed_resource_is_refused() {
    let rss = "NAME STRS\nSTRUCT S { BUF8 t; }\nSTRUCT U { BUF t; }\n\
               RESOURCE S { t = \"a\"; }\nRESOURCE U { t = \"Hello world, hello\"; }\n";
    let bytes = compile_rss(rss);
    assert_eq!(bytes[19], 0b10, "resource 2 is packed");
    assert_eq!(read(&Bytes(bytes), 1), Err(Unreadable::Packed { resource: 2 }));
}

#[test]
fn an_index_outside_the_file_is_refused() {
    let (_, mut bytes) = strings(&[("a".into(), "b".into())]);
    let size = bytes.len() as u32;
    let n = bytes.len();
    let at = size as u16 + 10;
    bytes[n - 2..].copy_from_slice(&at.to_le_bytes());
    let index = Unreadable::Index { at: u32::from(at), size };
    assert_eq!(read(&Bytes(bytes.clone()), 2), Err(index));
    // An odd tail, and an index that would overlap the header, are the same error.
    let odd = size as u16 - 3;
    bytes[n - 2..].copy_from_slice(&odd.to_le_bytes());
    let index = Unreadable::Index { at: u32::from(odd), size };
    assert_eq!(read(&Bytes(bytes.clone()), 2), Err(index));
    bytes[n - 2..].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(read(&Bytes(bytes), 2), Err(Unreadable::Index { at: 2, size }));
}

#[test]
fn an_entry_that_runs_backwards_or_past_the_index_is_refused() {
    // Two keys, so resource 2's end is resource 3's start and not the index's own
    // offset in the last two bytes.
    let (_, bytes) = strings(&[("a".into(), "bc".into()), ("d".into(), "e".into())]);
    let n = bytes.len();
    let index_at = usize::from(u16::from_le_bytes([bytes[n - 2], bytes[n - 1]]));
    let entry = |i: usize| u16::from_le_bytes([bytes[index_at + 2 * i], bytes[index_at + 2 * i + 1]]);
    let (start, end) = (entry(1), entry(2));
    let mut backwards = bytes.clone();
    backwards[index_at + 2..index_at + 4].copy_from_slice(&(end + 1).to_le_bytes());
    let found = Unreadable::Entry { resource: 2, start: u32::from(end) + 1, end: u32::from(end) };
    assert_eq!(read(&Bytes(backwards), 2), Err(found));
    let mut past = bytes.clone();
    past[index_at + 4..index_at + 6].copy_from_slice(&(index_at as u16 + 1).to_le_bytes());
    let found = Unreadable::Entry { resource: 2, start: u32::from(start), end: index_at as u32 + 1 };
    assert_eq!(read(&Bytes(past), 2), Err(found));
}

#[test]
fn a_file_too_short_for_a_header_is_refused() {
    let file = Bytes(vec![0; 20]);
    assert_eq!(read(&file, 1), Err(Unreadable::TooShort { size: 20 }));
}
