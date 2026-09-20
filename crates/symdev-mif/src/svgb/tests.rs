use super::*;

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn encode(src: &str) -> Vec<u8> {
    Svgb::new(3)
        .unwrap()
        .encode(&SvgElement::parse(src).unwrap())
        .unwrap()
}

/// svgb-mif-spec.md §4.1, §4.2: the smallest document and the tree markers.
#[test]
fn header_and_tree_markers() {
    assert_eq!(
        encode("<svg xmlns=\"http://www.w3.org/2000/svg\"/>"),
        hex("ce 56 fa 03 00 e8 03 fe ff")
    );
    assert_eq!(
        encode("<svg xmlns=\"http://www.w3.org/2000/svg\"><g><g><rect/></g><rect/></g></svg>"),
        hex("ce 56 fa 03 00 e8 03 0b e8 03 0b e8 03 21 e8 03 fe fe 21 e8 03 fe fe fe ff")
    );
}

/// svgb-mif-spec.md §8: the shipped icon template, byte for byte.
#[test]
fn symdev_icon_template() {
    let src = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
        <!-- symdev GUI template: app icon (SVG Tiny, built into gui_aif.mif). -->\n\
        <svg baseProfile=\"tiny\" xmlns=\"http://www.w3.org/2000/svg\" width=\"88\" height=\"88\" viewBox=\"0 0 88 88\">\n\
        <rect x=\"4\" y=\"4\" width=\"80\" height=\"80\" rx=\"18\" ry=\"18\" fill=\"#1f6feb\"/>\n\
        <rect x=\"22\" y=\"22\" width=\"44\" height=\"30\" rx=\"4\" ry=\"4\" fill=\"#ffffff\"/>\n\
        <rect x=\"30\" y=\"58\" width=\"28\" height=\"8\" rx=\"4\" ry=\"4\" fill=\"#ffffff\"/>\n\
        </svg>\n";
    let bytes = encode(src);
    assert_eq!(bytes.len(), 193);
    assert_eq!(
        bytes,
        hex("ce 56 fa 03 00 59 00 08 74 00 69 00 6e 00 79 00 \
             1a 00 00 00 00 58 00 1b 00 00 00 00 58 00 58 00 \
             00 00 00 00 00 00 00 00 00 00 58 00 00 00 58 00 \
             e8 03 21 31 00 00 00 04 00 30 00 00 00 04 00 1a \
             00 00 00 50 00 1b 00 00 00 50 00 1d 00 00 00 12 \
             00 1e 00 00 00 12 00 00 00 00 eb 6f 1f 00 e8 03 \
             fe 21 31 00 00 00 16 00 30 00 00 00 16 00 1a 00 \
             00 00 2c 00 1b 00 00 00 1e 00 1d 00 00 00 04 00 \
             1e 00 00 00 04 00 00 00 00 ff ff ff 00 e8 03 fe \
             21 31 00 00 00 1e 00 30 00 00 00 3a 00 1a 00 00 \
             00 1c 00 1b 00 00 00 08 00 1d 00 00 00 04 00 1e \
             00 00 00 04 00 00 00 00 ff ff ff 00 e8 03 fe fe \
             ff")
    );
}

/// svgb-mif-spec.md §8: the circle example. (The spec's snippet drops one byte of
/// the `fill` record; these bytes are what the real tool writes.)
#[test]
fn circle_and_view_box() {
    assert_eq!(
        encode(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 10\">\
             <circle cx=\"5\" cy=\"5\" r=\"4\" fill=\"#ffcc00\"/></svg>"
        ),
        hex(
            "ce 56 fa 03 00 58 00 00 00 00 00 00 00 00 00 00 00 0a 00 00 00 0a 00 e8 03 \
             1b 2e 00 00 00 05 00 2f 00 00 00 05 00 1c 00 00 00 04 00 00 00 00 00 cc ff 00 \
             e8 03 fe fe ff"
        )
    );
}

/// svgb-mif-spec.md §4.4, §4.6: truncation toward zero, the range check, the
/// `#rgb` expansion bug, `none`, and the version differences.
#[test]
fn numbers_and_colours_follow_the_spec() {
    let has = |bytes: &[u8], want: &str| {
        let want = hex(want);
        bytes.windows(want.len()).any(|w| w == want)
    };
    let x = |v: &str| encode(&format!("<svg><rect x=\"{v}\"/></svg>"));
    assert!(has(&x("0.1"), "31 00 99 19 00 00"));
    assert!(has(&x("-0.1"), "31 00 67 e6 ff ff"));
    assert!(has(&x("32765"), "31 00 00 00 fd 7f"));
    // Out of range: the attribute disappears and the rect has none.
    assert!(has(&x("32766"), "21 e8 03"));
    let fill = |v: &str| encode(&format!("<svg><rect fill=\"{v}\"/></svg>"));
    assert!(has(&fill("#abc"), "00 00 00 cf bf af 00"));
    assert!(has(&fill("none"), "00 00 00 ff ff ff 01"));
    assert!(has(&fill("rgb(1,2,3)"), "00 00 00 03 02 01 00"));
    assert!(has(&fill("url(#g)"), "00 00 01 02 67 00"));
    let v1 = Svgb::new(1)
        .unwrap()
        .encode(&SvgElement::parse("<svg><rect width=\"30\" fill=\"#123456\"/></svg>").unwrap())
        .unwrap();
    assert!(v1.starts_with(&hex("cc 56 fa 03")));
    assert!(has(&v1, "00 00 f0 41"));
    assert!(has(&v1, "12 34 56 00"));
}
