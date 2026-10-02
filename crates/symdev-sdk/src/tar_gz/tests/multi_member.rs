//! A `.tar.gz` may hold several gzip members (`pigz`, or archives concatenated): every
//! member is extracted, none is silently dropped.

use std::fs;
use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;

use super::URL;
use crate::TarGz;

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(bytes).unwrap();
    gz.finish().unwrap()
}

#[test]
fn every_gzip_member_is_extracted() {
    let mut tar = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(1);
    header.set_mode(0o644);
    tar.append_data(&mut header.clone(), "first", &b"1"[..])
        .unwrap();
    // The first member ends at an entry boundary, so a reader that stops there sees
    // what looks like a whole archive.
    let split = tar.get_ref().len();
    tar.append_data(&mut header, "second", &b"2"[..]).unwrap();
    let whole = tar.into_inner().unwrap();
    let members = [gzip(&whole[..split]), gzip(&whole[split..])].concat();
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("x.tar.gz");
    fs::write(&path, members).unwrap();
    let into = tmp.path().join("into");
    fs::create_dir(&into).unwrap();
    TarGz::new(&path, URL).extract(&into).unwrap();
    assert_eq!(fs::read(into.join("first")).unwrap(), b"1");
    assert_eq!(fs::read(into.join("second")).unwrap(), b"2");
}
