use crate::Rcomp;

fn unhex(s: &str) -> Vec<u8> {
    let s: String = s.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// Experiment 56 probes: our own `.rss`, compiled by SDK `rcomp.exe -u` under Wine.
#[test]
fn probes_match_wine_rcomp() {
    let cases: [(&str, &str, &str, &str); 10] = [
        (
            "multiple_resources_mix_raw_and_packed",
            include_str!("../testdata/exp56_multiple_resources_mix_raw_and_packed.rss"),
            include_str!("../testdata/exp56_multiple_resources_mix_raw_and_packed.rsc.hex"),
            include_str!("../testdata/exp56_multiple_resources_mix_raw_and_packed.rsg"),
        ),
        (
            "long_text_bare_buf_empty_and_run_lengths",
            include_str!("../testdata/exp56_long_text_bare_buf_empty_and_run_lengths.rss"),
            include_str!("../testdata/exp56_long_text_bare_buf_empty_and_run_lengths.rsc.hex"),
            include_str!("../testdata/exp56_long_text_bare_buf_empty_and_run_lengths.rsg"),
        ),
        (
            "len_byte_struct_array_and_embedded_struct",
            include_str!("../testdata/exp56_len_byte_struct_array_and_embedded_struct.rss"),
            include_str!("../testdata/exp56_len_byte_struct_array_and_embedded_struct.rsc.hex"),
            include_str!("../testdata/exp56_len_byte_struct_array_and_embedded_struct.rsg"),
        ),
        (
            "character_constant",
            include_str!("../testdata/exp56_character_constant.rss"),
            include_str!("../testdata/exp56_character_constant.rsc.hex"),
            include_str!("../testdata/exp56_character_constant.rsg"),
        ),
        (
            "negative_long",
            include_str!("../testdata/exp56_negative_long.rss"),
            include_str!("../testdata/exp56_negative_long.rsc.hex"),
            include_str!("../testdata/exp56_negative_long.rsg"),
        ),
        (
            "word_array_default",
            include_str!("../testdata/exp56_word_array_default.rss"),
            include_str!("../testdata/exp56_word_array_default.rsc.hex"),
            include_str!("../testdata/exp56_word_array_default.rsg"),
        ),
        (
            "byte_array_word_count",
            include_str!("../testdata/exp56_byte_array_word_count.rss"),
            include_str!("../testdata/exp56_byte_array_word_count.rsc.hex"),
            include_str!("../testdata/exp56_byte_array_word_count.rsg"),
        ),
        (
            "integer_expression",
            include_str!("../testdata/exp56_integer_expression.rss"),
            include_str!("../testdata/exp56_integer_expression.rsc.hex"),
            include_str!("../testdata/exp56_integer_expression.rsg"),
        ),
        (
            "compress_only_when_shorter",
            include_str!("../testdata/exp56_compress_only_when_shorter.rss"),
            include_str!("../testdata/exp56_compress_only_when_shorter.rsc.hex"),
            include_str!("../testdata/exp56_compress_only_when_shorter.rsg"),
        ),
        (
            "double",
            include_str!("../testdata/exp56_double.rss"),
            include_str!("../testdata/exp56_double.rsc.hex"),
            include_str!("../testdata/exp56_double.rsg"),
        ),
    ];
    for (name, rss, rsc, rsg) in cases {
        let compiled = Rcomp::compile(rss.as_bytes(), name).unwrap();
        assert_eq!(compiled.rsc_bytes().unwrap(), unhex(rsc), "{name}.rsc");
        assert_eq!(compiled.rsg_text(), rsg, "{name}.rsg");
    }
}

#[test]
fn name_value_is_base_27_letters_with_digits_zero() {
    use crate::RssCompiler;
    // Experiment 56: `TEST` → 0x6120e, `AB` → 29, `A1` → 27, `L10N` → 0x39ab2.
    assert_eq!(RssCompiler::name_value("TEST").unwrap(), 0x6120e);
    assert_eq!(RssCompiler::name_value("AB").unwrap(), 29);
    assert_eq!(RssCompiler::name_value("A1").unwrap(), 27);
    assert_eq!(RssCompiler::name_value("L10N").unwrap(), 0x39ab2);
    assert_eq!(RssCompiler::name_value("ZZZZ").unwrap(), 0x81bf0);
}

#[test]
fn undefined_name_in_text_is_its_spelling_and_control_chars_are_quoted() {
    // Experiment 56: helloworldbasic's missing `.rls` names; richtexteditor's "\f".
    let src = "NAME TEST\nSTRUCT T { LTEXT t; }\nRESOURCE T r_a { t = STRING_x; }\n\
               RESOURCE T r_b { t = \"a\\fb\"; }\n";
    let c = Rcomp::compile(src.as_bytes(), "t").unwrap();
    let rsc = c.rsc_bytes().unwrap();
    assert!(rsc.windows(8).any(|w| w == b"STRING_x"));
    assert!(rsc.windows(4).any(|w| w == [b'a', 0x01, 0x0c, b'b']));
}

/// rcomp-spec.md §4: an unassigned `LINK`/`LLINK`/`BUF` writes nothing at all, while
/// the fixed-size types write their zeros.
#[test]
fn unassigned_members_match_the_spec() {
    let src = "NAME TEST\nSTRUCT S { BYTE b; WORD w; LONG l; LTEXT t; BUF u; LINK k; LLINK m; }\n\
               RESOURCE S r_a { }\n";
    let c = Rcomp::compile(src.as_bytes(), "t").unwrap();
    assert_eq!(c.resources[0].data.uncompressed(), [0u8; 8]);
}

/// rcomp-spec.md §5.1: out-of-range integers are refused, negatives are two's complement.
#[test]
fn integer_range_follows_the_spec() {
    let one = |v: &str| {
        Rcomp::compile(
            format!("NAME TEST\nSTRUCT S {{ BYTE b; }}\nRESOURCE S r_a {{ b = {v}; }}\n")
                .as_bytes(),
            "t",
        )
        .map(|c| c.resources[0].data.uncompressed())
    };
    assert_eq!(one("255").unwrap(), [0xff]);
    assert_eq!(one("-128").unwrap(), [0x80]);
    assert!(one("256").is_err());
    assert!(one("-129").is_err());
}

/// rcomp-spec.md §5.6: source bytes are CP1252 unless `CHARACTER_SET` says otherwise.
#[test]
fn source_characters_are_cp1252_by_default() {
    let body = b"NAME TEST\nSTRUCT S { BUF b; }\nRESOURCE S r_a { b = \"\x80\x91\x9e\xe9\"; }\n";
    let c = Rcomp::compile(body, "t").unwrap();
    assert_eq!(
        c.resources[0].data.uncompressed(),
        [0xac, 0x20, 0x18, 0x20, 0x7e, 0x01, 0xe9, 0x00]
    );
    let latin1 = [b"CHARACTER_SET ISOLATIN1\n".as_slice(), body].concat();
    let c = Rcomp::compile(&latin1, "t").unwrap();
    assert_eq!(
        c.resources[0].data.uncompressed(),
        [0x80, 0x00, 0x91, 0x00, 0x9e, 0x00, 0xe9, 0x00]
    );
}
