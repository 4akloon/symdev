//! `compile_icon_container`: the three files of a container, against the bytes
//! `mifconv`/`bmconv` were seen to write (experiment 64).
use std::path::{Path, PathBuf};

use symdev_manifest::{IconContainer, IconSource};
use symdev_mbm::{BmpImage, MbmBitmap, MbmDepth, MbmFile};

use super::fake;
use crate::icons::IconOutputs;

/// A 1×1 24-bit BMP of one colour.
fn bmp(r: u8, g: u8, b: u8) -> Vec<u8> {
    let mut out = b"BM".to_vec();
    out.extend_from_slice(&58u32.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&54u32.to_le_bytes());
    for word in [40u32, 1, 1] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    for word in [0u32, 4, 0, 0, 0, 0] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&[b, g, r, 0]);
    out
}

fn source(file: &str, depth: &str, animated: bool) -> IconSource {
    IconSource {
        file: PathBuf::from(file),
        depth: depth.into(),
        animated,
    }
}

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("gfx")).unwrap();
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("gfx/red.bmp"), bmp(255, 0, 0)).unwrap();
    std::fs::write(dir.path().join("gfx/red_mask_soft.bmp"), bmp(9, 9, 9)).unwrap();
    std::fs::write(dir.path().join("gfx/blue.bmp"), bmp(0, 0, 255)).unwrap();
    std::fs::write(
        dir.path().join("gfx/app.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\"/>",
    )
    .unwrap();
    dir
}

fn compile(root: &Path, container: &IconContainer) -> Result<IconOutputs, String> {
    fake()
        .compile_icon_container(container, root, &root.join("build"))
        .map_err(|e| e.to_string())?;
    Ok(IconOutputs::of(container, &root.join("build")))
}

#[test]
fn bitmaps_go_to_the_mbm_and_the_mif_points_at_them() {
    let dir = project();
    let container = IconContainer {
        dest: "!:\\resource\\apps\\games.mif".into(),
        header: Some("puzzles.mbg".into()),
        sources: vec![
            source("gfx/red.bmp", "c24,8", false),
            source("gfx/app.svg", "c32,8", false),
            source("gfx/blue.bmp", "c8", false),
        ],
    };
    let out = compile(dir.path(), &container).unwrap();
    let mif = std::fs::read(out.mif()).unwrap();
    assert_eq!(&mif[..16], b"B##4\x02\0\0\0\x10\0\0\0\x06\0\0\0");
    // red: bitmap 0, its mask bitmap 1; the SVG block after the six entries; blue: 2.
    assert_eq!(&mif[16..24], [0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&mif[24..32], [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
    assert_eq!(&mif[32..36], 64u32.to_le_bytes());
    assert_eq!(&mif[48..56], [0xfe, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
    assert_eq!(&mif[64..68], b"C##4");
    let want = MbmFile::new(vec![
        MbmBitmap::compile(
            &BmpImage::parse(&bmp(255, 0, 0)).unwrap(),
            MbmDepth::Colour24,
        ),
        MbmBitmap::compile(&BmpImage::parse(&bmp(9, 9, 9)).unwrap(), MbmDepth::Grey8),
        MbmBitmap::compile(
            &BmpImage::parse(&bmp(0, 0, 255)).unwrap(),
            MbmDepth::Colour8,
        ),
    ])
    .bytes()
    .unwrap();
    assert_eq!(std::fs::read(out.mbm().unwrap()).unwrap(), want);
    assert_eq!(
        std::fs::read_to_string(out.mbg().unwrap()).unwrap(),
        " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMifPuzzles\r\n\t{\r\n\
         \tEMbmPuzzlesRed = 16384,\r\n\tEMbmPuzzlesRed_mask = 16385,\r\n\tEMbmPuzzlesApp = 16386,\r\n\
         \tEMbmPuzzlesApp_mask = 16387,\r\n\tEMbmPuzzlesBlue = 16388,\r\n\tEMbmPuzzlesLastElement\r\n\t};\r\n"
    );
}

#[test]
fn an_svg_only_container_writes_no_mbm() {
    let dir = project();
    let container = IconContainer {
        dest: "!:\\resource\\apps\\0xa000ef77\\puzzles.mif".into(),
        header: None,
        sources: vec![source("gfx/app.svg", "c32,8", false)],
    };
    let out = compile(dir.path(), &container).unwrap();
    assert!(out.mif().is_file());
    assert!(!dir.path().join("build/puzzles.mbm").exists());
    assert!(!dir.path().join("build/puzzles.mbg").exists());
}

#[test]
fn the_unobserved_bitmap_forms_are_refused_by_name() {
    let dir = project();
    for (depth, animated, expect) in [
        ("c24,1", false, "not observed"),
        ("c24", true, "not observed"),
        ("c32", false, "unknown bitmap depth"),
        ("c24,8", false, "blue_mask_soft.bmp"),
    ] {
        let container = IconContainer {
            dest: "!:\\resource\\apps\\x.mif".into(),
            header: None,
            sources: vec![source("gfx/blue.bmp", depth, animated)],
        };
        let err = compile(dir.path(), &container).unwrap_err();
        assert!(err.contains(expect), "{depth}/{animated}: {err}");
    }
}
