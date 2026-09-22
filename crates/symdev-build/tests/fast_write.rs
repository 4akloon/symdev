use core::fmt::Write;
use symbian_fmt::write;
#[test]
fn smoke() {
    let mut s = String::new();
    let n = -5;
    write!(s, "a{}b{n}", 7u8).unwrap();
    assert_eq!(s, "a7b-5");
}
