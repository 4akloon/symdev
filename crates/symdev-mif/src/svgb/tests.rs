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

mod paths;

/// Experiment 59: the `<svg>` attributes Adobe Illustrator's SVG Tiny export
/// writes, byte for byte against `svgtbinencode -v 3`.
#[test]
fn illustrator_svg_header() {
    let src = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
        <!-- Generator: Adobe Illustrator 16.0.0 -->\n\
        <!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1 Tiny//EN\" \
          \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11-tiny.dtd\">\n\
        <svg version=\"1.1\" id=\"Layer_1\" xmlns=\"http://www.w3.org/2000/svg\" \
          xmlns:xlink=\"http://www.w3.org/1999/xlink\" x=\"0px\" y=\"0px\" \
          xml:space=\"preserve\"/>";
    assert_eq!(
        encode(src),
        hex("ce 56 fa 03 00 4c 00 99 19 01 00 \
             5c 00 0e 4c 00 61 00 79 00 65 00 72 00 5f 00 31 00 \
             31 00 00 00 00 00  30 00 00 00 00 00 \
             5f 00 10 70 00 72 00 65 00 73 00 65 00 72 00 76 00 65 00 \
             e8 03 fe ff")
    );
}

/// Experiment 59: `style` becomes the same records as the attributes would,
/// in declaration order and at the position `style` itself occupies.
#[test]
fn style_is_parsed_into_records() {
    assert_eq!(
        encode("<svg><rect style=\"fill:red;stroke-width:2\" x=\"1\"/></svg>"),
        hex("ce 56 fa 03 00 e8 03 21 \
             00 00 00 00 00 ff 00  02 00 00 00 02 00  31 00 00 00 01 00 \
             e8 03 fe fe ff")
    );
    // A property the tool does not know carries no bytes at all.
    assert_eq!(
        encode("<svg><rect style=\"foo:bar;enable-background:new 0 0 1 1\"/></svg>"),
        hex("ce 56 fa 03 00 e8 03 21 e8 03 fe fe ff")
    );
}

/// Experiment 59: `rgb()` percentages are scaled by the `float` 2.55, so
/// 100 % is 254; the keyword table is matched case-insensitively.
#[test]
fn colour_percentages_and_keywords() {
    assert_eq!(
        encode("<svg><rect fill=\"rgb(100%,50%,25%)\" stroke=\"DarkSlateGray\"/></svg>"),
        hex("ce 56 fa 03 00 e8 03 21 00 00 00 3f 7f fe 00 01 00 4f 4f 2f 00 e8 03 fe fe ff")
    );
    // `grey` is not in the tool's table; we refuse rather than write black.
    assert!(
        Svgb::new(3)
            .unwrap()
            .encode(&SvgElement::parse("<svg><rect fill=\"grey\"/></svg>").unwrap())
            .is_err()
    );
}

/// Experiment 59: `<use>` writes the reference and then the target id, the
/// `#` stripped only when an element already carries that id.
#[test]
fn use_resolves_its_reference() {
    assert_eq!(
        encode("<svg><rect id=\"a\"/><use xlink:href=\"#a\"/></svg>"),
        hex("ce 56 fa 03 00 e8 03 21 5c 00 02 61 00 e8 03 fe \
             1a 6d 00 04 23 00 61 00 02 61 00 e8 03 fe fe ff")
    );
    // Defined after the `<use>`: the reference is repeated unchanged.
    assert_eq!(
        encode("<svg><use xlink:href=\"#a\"/><rect id=\"a\"/></svg>"),
        hex(
            "ce 56 fa 03 00 e8 03 1a 6d 00 04 23 00 61 00 04 23 00 61 00 e8 03 fe \
             21 5c 00 02 61 00 e8 03 fe fe ff"
        )
    );
}

/// Experiment 59: only `<text>` keeps its character data, and only
/// `xml:space="preserve"` on that element itself stops the collapsing.
#[test]
fn character_data_and_xml_space() {
    assert_eq!(
        encode("<svg><text>  a   b  </text></svg>"),
        hex("ce 56 fa 03 00 e8 03 19 e8 03 fd 06 61 00 20 00 62 00 fe fe ff")
    );
    assert_eq!(
        encode("<svg><title>dropped</title></svg>"),
        hex("ce 56 fa 03 00 e8 03 07 e8 03 fe fe ff")
    );
    let preserved = encode("<svg><text xml:space=\"preserve\">  a\n\tb  </text></svg>");
    assert!(preserved.ends_with(&hex(
        "fd 10 20 00 20 00 61 00 20 00 20 00 62 00 20 00 20 00 fe fe ff"
    )));
}

/// Experiment 59: every number is quantised to 16.16 first, so versions 1
/// and 4 write the quantised value as a float, not the value that was typed.
#[test]
fn float_versions_write_the_quantised_number() {
    let v1 = Svgb::new(1)
        .unwrap()
        .encode(&SvgElement::parse("<svg><rect x=\"0.1\"/></svg>").unwrap())
        .unwrap();
    assert_eq!(
        v1,
        hex("cc 56 fa 03 00 e8 03 21 31 00 00 c8 cc 3d e8 03 fe fe ff")
    );
}

/// Values the tool mishandles, which this encoder refuses instead.
#[test]
fn refuses_what_the_tool_gets_wrong() {
    let bad = |src: &str| {
        Svgb::new(3)
            .unwrap()
            .encode(&SvgElement::parse(src).unwrap())
            .is_err()
    };
    assert!(bad("<svg><rect x=\"+5\"/></svg>"));
    assert!(bad("<svg><rect opacity=\".25\"/></svg>"));
    assert!(bad("<svg viewBox=\"0,0,10,10\"/>"));
    assert!(bad("<svg><rect preserveAspectRatio=\"none\"/></svg>"));
    assert!(bad("<svg><rect transform=\"rotate(45)\"/></svg>"));
    assert!(bad("<svg><rect style=\"d:M0 0\"/></svg>"));
    assert!(bad("<svg><foreignObject/></svg>"));
}
