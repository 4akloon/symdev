use crate::model::{Mmp, MmpResource};

#[test]
fn parse_exe_with_source() {
    let m = Mmp::parse("TARGET hello.exe\nTARGETTYPE EXE\nSOURCE hello.cpp\n").unwrap();
    assert_eq!(m.target, "hello.exe");
    assert_eq!(m.source, ["hello.cpp"]);
}

#[test]
fn dll_accepted_other_types_rejected() {
    let m = Mmp::parse("TARGET m.dll\nTARGETTYPE DLL\nUID 0x1000008d 0xe5d1b001\nSOURCE m.cpp\n")
        .unwrap();
    assert_eq!(m.target_type, "DLL");
    assert_eq!(m.uid, [0x1000_008d, 0xe5d1_b001]);
    assert!(Mmp::parse("TARGET x.dll\nTARGETTYPE PLUGIN\nSOURCE a.cpp\n").is_err());
}

#[test]
fn uid_two_hex_values() {
    let m = Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x100039CE 0x1000008d\nSOURCE a.cpp\n")
        .unwrap();
    assert_eq!(m.uid, [0x1000_39CE, 0x1000_008d]);
}

#[test]
fn uid_three_values_hex_and_decimal() {
    let m =
        Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x1000007a 100 0xE0000001\nSOURCE a.cpp\n")
            .unwrap();
    assert_eq!(m.uid, [0x1000_007a, 100, 0xE000_0001]);
}

#[test]
fn uid_one_value_rejected() {
    assert!(Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nUID 0x1000007a\nSOURCE a.cpp\n").is_err());
}

#[test]
fn uid_omitted() {
    let m = Mmp::parse("TARGET hello.exe\nTARGETTYPE EXE\nSOURCE hello.cpp\n").unwrap();
    assert!(m.uid.is_empty());
}

#[test]
fn start_resource_block_is_typed() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCEPATH ..\\src\nSOURCE a.cpp\nSOURCEPATH ..\\data\nSTART RESOURCE hello.rss\nHEADER\nTARGETPATH \\resource\\apps\nEND\n",
    )
    .unwrap();
    assert_eq!(
        m.resource,
        [MmpResource {
            file: "hello.rss".into(),
            sourcepath: Some("..\\data".into()),
            targetpath: Some("\\resource\\apps".into()),
            header: true,
            lang: Vec::new(),
        }]
    );
    assert!(m.targetpath.is_none());
}

#[test]
fn start_resource_rejects_unobserved_directive() {
    assert!(
        Mmp::parse(
            "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nSTART RESOURCE a.rss\nUID 1 2\nEND\n"
        )
        .is_err()
    );
}

#[test]
fn each_source_keeps_the_sourcepath_in_effect() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCEPATH ..\\src\nSOURCE a.cpp\nSOURCEPATH ..\\data\nSTART RESOURCE a.rss\nEND\nSOURCE b.cpp\n",
    )
    .unwrap();
    assert_eq!(m.source, ["a.cpp", "b.cpp"]);
    assert_eq!(
        m.source_sourcepath,
        [Some("..\\src".to_string()), Some("..\\data".to_string())]
    );
}

#[test]
fn a_line_marker_names_the_file_in_the_error() {
    let err = Mmp::parse("# 1 \"gui.mmp\"\nTARGET x.exe\nTARGETTYPE EXE\nWHAT 1\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("gui.mmp:3"), "{err}");
}

#[test]
fn macro_and_option_reach_the_model_in_order() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\n\
         MACRO COMBINED\nMACRO STYLUS_BASED NO_TGZ=1\n\
         OPTION GCCE -O3\nOPTION gcce -fno-strict-aliasing\nOPTION ARMCC --diag_suppress 1234\n",
    )
    .unwrap();
    assert_eq!(m.macros, ["COMBINED", "STYLUS_BASED", "NO_TGZ=1"]);
    assert_eq!(m.option("GCCE"), Some("-O3 -fno-strict-aliasing"));
    assert_eq!(m.option("ARMCC"), Some("--diag_suppress 1234"));
    assert_eq!(m.option("WINSCW"), None);
}

#[test]
fn secureid_and_a_zero_vendorid_parse_and_a_real_one_is_refused() {
    let m =
        Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nSECUREID 0xE7351C20\nVENDORID 0\n")
            .unwrap();
    assert_eq!(m.secureid, Some(0xe735_1c20));
    assert_eq!(m.vendorid, Some(0));
    let err = Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nVENDORID 0x70000001\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("--vid"), "{err}");
}

#[test]
fn lang_is_recorded_and_a_dead_directive_only_warns() {
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nLANG SC 01\n\
         DEBUGGABLE_UDEBONLY\nEPOCSTACKSIZE 0x14000\n",
    )
    .unwrap();
    assert_eq!(m.lang, ["SC", "01"]);
    assert_eq!(m.warnings.len(), 2);
    assert!(
        m.warnings[0].contains("DEBUGGABLE_UDEBONLY"),
        "{:?}",
        m.warnings
    );
}

#[test]
fn an_assp_directive_and_the_flat_resource_form_are_refused() {
    for line in [
        "ASSPLIBRARY x.lib",
        "RESOURCE gui.rss",
        "AIF a b c",
        "SYSTEMRESOURCE x.rss",
    ] {
        let text = format!("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\n{line}\n");
        assert!(Mmp::parse(&text).is_err(), "{line} was accepted");
    }
}

#[test]
fn capability_names_are_case_insensitive_and_all_expands() {
    use crate::MmpCapabilities;
    let m = Mmp::parse(
        "TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nCAPABILITY readuserdata NetworkServices\n",
    )
    .unwrap();
    let caps = MmpCapabilities::of(&m).unwrap();
    assert_eq!(caps.names(), ["NetworkServices", "ReadUserData"]);
    caps.check(&["ReadUserData".into(), "NetworkServices".into()], "x.exe")
        .unwrap();
    assert!(caps.check(&["ReadUserData".into()], "x.exe").is_err());

    let all =
        Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nCAPABILITY ALL -TCB\n").unwrap();
    let all = MmpCapabilities::of(&all).unwrap();
    assert_eq!(all.names().len(), 19);
    assert!(!all.names().contains(&"TCB"));

    let none = Mmp::parse("TARGET x.exe\nTARGETTYPE EXE\nSOURCE a.cpp\nCAPABILITY NONE\n").unwrap();
    assert!(MmpCapabilities::of(&none).unwrap().names().is_empty());
}
