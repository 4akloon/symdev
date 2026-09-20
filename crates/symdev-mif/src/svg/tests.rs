use super::*;

#[test]
fn reads_elements_and_attributes_in_document_order() {
    let doc = SvgElement::parse(
        "<?xml version=\"1.0\"?>\n<!-- c -->\n<svg baseProfile=\"tiny\" width=\"88\">\n\
         <g><rect x=\"4\" fill=\"#1f6feb\"/></g>\n<circle r='4'/>\n</svg>\n",
    )
    .unwrap();
    assert_eq!(doc.name, "svg");
    assert_eq!(doc.attributes[0], ("baseProfile".into(), "tiny".into()));
    assert_eq!(doc.attribute("width"), Some("88"));
    assert_eq!(doc.children[0].children[0].name, "rect");
    assert_eq!(
        doc.children[0].children[0].attribute("fill"),
        Some("#1f6feb")
    );
    assert_eq!(doc.children[1].name, "circle");
}

#[test]
fn rejects_text_and_unterminated_markup() {
    assert!(SvgElement::parse("<svg><text>hi</text></svg>").is_err());
    assert!(SvgElement::parse("<svg>").is_err());
}
