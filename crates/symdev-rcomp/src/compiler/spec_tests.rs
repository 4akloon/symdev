//! The test vectors of [rcomp-spec.md](../../../../docs/research/rcomp-spec.md) §6.

use crate::Rcomp;

fn unhex(s: &str) -> Vec<u8> {
    let s: String = s.split_whitespace().collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// The image and header facts of one resource of a compiled source.
fn image(src: &[u8], index: usize) -> (Vec<u8>, bool, usize) {
    let c = Rcomp::compile(src, "t").unwrap();
    let data = &c.resources[index].data;
    let largest = c.resources.iter().map(|r| r.data.len()).max().unwrap_or(0);
    match data.packed().unwrap() {
        Some(bytes) => (bytes, true, largest),
        None => (data.uncompressed(), false, largest),
    }
}

/// rcomp-spec.md §6: every test vector, one row per rule.
#[test]
fn spec_test_vectors() {
    let s = |body: &str| format!("STRUCT AA {{ {} }}\n", body);
    let b = |src: String| src.into_bytes();
    // The two CP1252 rows need real 8-bit source bytes, not UTF-8.
    let high = |src: String| {
        src.replace("@@", "\u{1}\u{2}\u{3}\u{4}")
            .bytes()
            .map(|b| match b {
                1 => 0x80,
                2 => 0x91,
                3 => 0x9e,
                4 => 0xe9,
                other => other,
            })
            .collect::<Vec<u8>>()
    };
    let rows: [(Vec<u8>, &str, bool, usize); 21] = [
        (
            b(s("LTEXT a; }\nRESOURCE AA r1 { a = \"Hello\";")),
            "00 01 05 05 48 65 6c 6c 6f",
            true,
            12,
        ),
        (
            b(s("BUF a; }\nRESOURCE AA r1 { a = \"Hello\";")),
            "05 48 65 6c 6c 6f",
            true,
            10,
        ),
        (
            b(s("LTEXT8 a; }\nRESOURCE AA r1 { a = \"Hello\";")),
            "05 48 65 6c 6c 6f",
            false,
            6,
        ),
        (
            b(s("BUF8 a; }\nRESOURCE AA r1 { a = \"Hello\";")),
            "48 65 6c 6c 6f",
            false,
            5,
        ),
        (
            b(s("BUF a; WORD b; }\nRESOURCE AA r1 { a = \"Hi\"; b=7;")),
            "48 00 69 00 07 00",
            false,
            6,
        ),
        (
            b(s("BYTE b; BUF a; }\nRESOURCE AA r1 { b=0x12; a=\"Hello\";")),
            "00 01 12 05 48 65 6c 6c 6f",
            true,
            12,
        ),
        (
            b(s("BYTE b; BUF a; }\nRESOURCE AA r1 { b=0x12; a=<0x0431>;")),
            "12 ab 31 04",
            false,
            4,
        ),
        (
            b(s(
                "BUF a; BUF b; }\nRESOURCE AA r1 { a=\"Hello\"; b=\"World\";",
            )),
            "05 48 65 6c 6c 6f 00 05 57 6f 72 6c 64",
            true,
            20,
        ),
        (
            b(s(
                "BUF a; WORD b; }\nRESOURCE AA r1 { a=\"Hello\"; b=0x1234;",
            )),
            "05 48 65 6c 6c 6f 02 34 12",
            true,
            12,
        ),
        (
            b(s(
                "WORD b; BUF a; }\nRESOURCE AA r1 { b=0x1234; a=\"Hello\";",
            )),
            "00 02 34 12 05 48 65 6c 6c 6f",
            true,
            12,
        ),
        (
            b(s(
                "LTEXT a; LTEXT b; }\nRESOURCE AA r1 { a=\"Hello\"; b=\"World\";",
            )),
            "00 01 05 05 48 65 6c 6c 6f 01 05 05 57 6f 72 6c 64",
            true,
            24,
        ),
        (
            b(s("LTEXT a[]; }\nRESOURCE AA r1 { a = {\"Hi\",\"Yo\"};")),
            "00 03 02 00 02 02 48 69 01 02 02 59 6f",
            true,
            14,
        ),
        (
            b(s(
                "LTEXT a[]; }\nRESOURCE AA r1 { a = {\"2\",\"2\",\"2\",\"2\",\"2\",\"2\",\"2\",\"2\",\"2\"};",
            )),
            "00 23 09 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 01 ab 32 00 \
             01 ab 32 00 01 ab 32 00 01 01 32",
            true,
            38,
        ),
        (
            b(s("LTEXT a; }\nRESOURCE AA r1 { a = \"A\" <0x0431> \"B\";")),
            "00 01 03 04 41 03 b1 42",
            true,
            8,
        ),
        (
            b(s("LTEXT a; }\nRESOURCE AA r1 { a = \"AA\\fBB\";")),
            "00 01 05 06 41 41 01 0c 42 42",
            true,
            12,
        ),
        (
            b(s("LTEXT a; }\nRESOURCE AA r1 { a = <0x0431>;")),
            "01 ab 31 04",
            false,
            4,
        ),
        (
            b(s(
                "BYTE b; WORD w; LONG l; LTEXT t; BUF u; LINK k; LLINK m; }\nRESOURCE AA r1 { ",
            )),
            "00 00 00 00 00 00 00 00",
            false,
            8,
        ),
        (
            b(s(
                "SRLINK s; WORD w; }\nRESOURCE AA r1 { w=1; }\nRESOURCE AA r2 { w=2;",
            )),
            "01 00 00 00 01 00",
            false,
            6,
        ),
        (
            b(s("DOUBLE a; }\nRESOURCE AA r1 { a=1.5;")),
            "00 00 00 00 00 00 f8 3f",
            false,
            8,
        ),
        (
            high(format!(
                "CHARACTER_SET ISOLATIN1\n{}",
                s("LTEXT a; }\nRESOURCE AA r1 { a = \"@@\";")
            )),
            "00 01 04 04 80 91 9e e9",
            true,
            10,
        ),
        (
            high(s("LTEXT a; }\nRESOURCE AA r1 { a = \"@@\";")),
            "04 ab ac 20 18 20 7e 01 e9 00",
            false,
            10,
        ),
    ];
    for (i, (src, want, packed, largest)) in rows.iter().enumerate() {
        let (bytes, is_packed, size) = image(src, 0);
        assert_eq!(bytes, unhex(want), "vector {} image", i + 1);
        assert_eq!(is_packed, *packed, "vector {} packed", i + 1);
        assert_eq!(size, *largest, "vector {} largest uncompressed", i + 1);
    }
    // Vector 18 also pins the second resource's own id in its SRLINK.
    let (second, _, _) = image(
        &b(s(
            "SRLINK s; WORD w; }\nRESOURCE AA r1 { w=1; }\nRESOURCE AA r2 { w=2;",
        )),
        1,
    );
    assert_eq!(second, unhex("02 00 00 00 02 00"));
}

/// rcomp-spec.md §6: the whole file for vector 1, header and index included.
#[test]
fn spec_full_file_and_rsg_line() {
    let src = "STRUCT AA { LTEXT a; }\nRESOURCE AA r1 { a = \"Hello\"; }\n";
    let c = Rcomp::compile(src.as_bytes(), "t").unwrap();
    assert_eq!(
        c.rsc_bytes().unwrap(),
        unhex(
            "6b 4a 1f 10 00 00 00 00 00 00 00 00 19 fd 48 e8 \
             01 0c 00 01 00 01 05 05 48 65 6c 6c 6f 14 00 1d 00"
        )
    );
    assert_eq!(
        c.rsg_text(),
        "#define R1                                        1\r\n"
    );
}

/// rcomp-spec.md §4: the name is padded into a 41-character field, then one space.
#[test]
fn rsg_pads_to_column_51_and_keeps_one_space_for_long_names() {
    let long = "r_this_is_a_very_long_resource_name_over_41ch";
    let src = format!(
        "STRUCT AA {{ BYTE b; }}\nRESOURCE AA r1 {{ b=1; }}\nRESOURCE AA {long} {{ b=2; }}\n"
    );
    let rsg = Rcomp::compile(src.as_bytes(), "t").unwrap().rsg_text();
    let lines: Vec<&str> = rsg.lines().collect();
    assert_eq!(lines[0], format!("#define R1{}1", " ".repeat(40)));
    assert_eq!(lines[1], format!("#define {} 2", long.to_uppercase()));
}
