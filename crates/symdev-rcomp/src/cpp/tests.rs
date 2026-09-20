use super::*;

#[test]
fn includes_case_insensitively_and_applies_conditionals_and_macros() {
    let dir = std::env::temp_dir().join(format!("symdev-rss-cpp-{}", std::process::id()));
    let sub = dir.join("inc");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join("shapes.rh"),
        "#ifndef SHAPES_RH\n#define SHAPES_RH\n#define KSide 4\n#define AREA(s) ((s)*(s))\n\
         STRUCT SQ { WORD side; WORD area; }\n#endif\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("app.rss"),
        "NAME TEST\n#include \"INC\\Shapes.RH\"\n#include <inc/shapes.rh>\n\
         #if defined(_UNICODE) && KSide > 3\nRESOURCE SQ r_sq { side = KSide; area = AREA(KSide); }\n\
         #else\n#error wrong branch\n#endif\n",
    )
    .unwrap();
    let rpp = CPreprocessor::for_rss(std::slice::from_ref(&dir))
        .run(&dir.join("app.rss"))
        .unwrap();
    let text = String::from_utf8(rpp.clone()).unwrap();
    assert!(text.contains("shapes.rh\" 1\n"), "{text}");
    let compiled = crate::Rcomp::compile(&rpp, "app.rss").unwrap();
    assert_eq!(compiled.resources.len(), 1);
    assert_eq!(compiled.resources[0].data.uncompressed(), [4, 0, 16, 0]);
    std::fs::remove_dir_all(&dir).unwrap();
}
