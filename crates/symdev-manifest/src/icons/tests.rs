use std::path::PathBuf;

use crate::tests::{HELLO, reject};
use crate::{IconContainer, IconSource, parse};

fn with(icons: &str) -> String {
    format!("{HELLO}\n{icons}")
}

#[test]
fn a_container_is_one_mifconv_call() {
    let m = parse(&with(
        "[[icons]]\ndest = \"/resource/apps/0xa000ef77/games.mif\"\nheader = \"puzzles_0xa000ef77.mbg\"\n\
         depth = \"c24\"\nsources = [\"gfx/blackbox.bmp\", { file = \"gfx/app.svg\", depth = \"c32,8\", animated = true }]\n",
    ))
    .unwrap();
    assert_eq!(
        m.icons,
        [IconContainer {
            dest: "!:\\resource\\apps\\0xa000ef77\\games.mif".into(),
            header: Some("puzzles_0xa000ef77.mbg".into()),
            sources: vec![
                IconSource {
                    file: PathBuf::from("gfx/blackbox.bmp"),
                    depth: "c24".into(),
                    animated: false,
                },
                IconSource {
                    file: PathBuf::from("gfx/app.svg"),
                    depth: "c32,8".into(),
                    animated: true,
                },
            ],
        }]
    );
    assert_eq!(m.icons[0].mif_name(), "games.mif");
}

#[test]
fn icons_are_optional_and_the_header_too() {
    assert!(parse(HELLO).unwrap().icons.is_empty());
    let m = parse(&with(
        "[[icons]]\ndest = \"\\\\resource\\\\apps\\\\x.mif\"\nsources = [{ file = \"a.svg\", depth = \"c32,8\" }]\n",
    ))
    .unwrap();
    assert_eq!(m.icons[0].header, None);
}

#[test]
fn every_source_needs_a_depth_from_somewhere() {
    reject(&with(
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\nsources = [\"a.svg\"]\n",
    ));
    reject(&with(
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"\"\nsources = [\"a.svg\"]\n",
    ));
}

#[test]
fn the_destination_names_a_mif_and_sources_are_svg_or_bmp() {
    for bad in [
        "[[icons]]\ndest = \"/resource/apps/x.mbm\"\ndepth = \"c24\"\nsources = [\"a.bmp\"]\n",
        "[[icons]]\ndest = \"resource/apps/x.mif\"\ndepth = \"c24\"\nsources = [\"a.bmp\"]\n",
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"c24\"\nsources = [\"a.png\"]\n",
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"c24\"\nsources = [\"/etc/a.bmp\"]\n",
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"c24\"\nsources = []\n",
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"c24\"\nheader = \"inc/x.mbg\"\nsources = [\"a.bmp\"]\n",
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\ndepth = \"c24\"\nsources = [{ file = \"a.bmp\", colour = 1 }]\n",
    ] {
        reject(&with(bad));
    }
}

#[test]
fn two_containers_cannot_share_a_file_name() {
    reject(&with(
        "[[icons]]\ndest = \"/resource/apps/a/x.mif\"\ndepth = \"c24\"\nsources = [\"a.bmp\"]\n\
         [[icons]]\ndest = \"/resource/apps/b/X.MIF\"\ndepth = \"c24\"\nsources = [\"b.bmp\"]\n",
    ));
    reject(&with(
        "[[icons]]\ndest = \"/resource/apps/x.mif\"\nheader = \"x.mbg\"\ndepth = \"c24\"\nsources = [\"a.bmp\"]\n\
         [[icons]]\ndest = \"/resource/apps/y.mif\"\nheader = \"x.mbg\"\ndepth = \"c24\"\nsources = [\"b.bmp\"]\n",
    ));
}
