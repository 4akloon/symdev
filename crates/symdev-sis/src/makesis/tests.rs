use super::*;

fn args(tokens: &[&str]) -> Vec<String> {
    tokens.iter().map(|t| (*t).to_string()).collect()
}

#[test]
fn from_args_match_experiment_7() {
    let m = Makesis::from_args(&args(&["makesis", "-v", "hello.pkg", "hello.sis"])).unwrap();
    assert!(m.verbose);
    assert_eq!(m.pkg, PathBuf::from("hello.pkg"));
    assert_eq!(m.sis, PathBuf::from("hello.sis"));
}

#[test]
fn from_args_todo_unused_flags() {
    let err = Makesis::from_args(&args(&["makesis", "-i", "hello.pkg", "hello.sis"])).unwrap_err();
    assert!(err.to_string().contains("TODO: makesis -i"));
}

#[test]
fn parse_wave0_pkg_matches_experiment_7() {
    let text = "&EN\r\n#{\"hello\"},(0xe79e4cf9),1,0,24,TYPE=SA\r\n%{\"Vendor-EN\"}\r\n:\"Vendor\"\r\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\r\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\r\n";
    let p = Wave0Pkg::parse(text).unwrap();
    assert_eq!(p.name, "hello");
    assert_eq!(p.uid3, 0xe79e_4cf9);
    assert_eq!(p.version, (1, 0, 24));
    assert_eq!(p.vendor, "Vendor");
    assert_eq!(p.vendor_localized, "Vendor-EN");
    assert_eq!(p.exe, PathBuf::from("hello.exe"));
    assert!(p.files.is_empty());
}

#[test]
fn parse_wave0_pkg_skips_comment_lines() {
    let text = "; header\r\n&EN\r\n; vendor\r\n#{\"hello\"},(0xe79e4cf9),1,0,24,TYPE=SA\r\n%{\"Vendor-EN\"}\r\n:\"Vendor\"\r\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\r\n; EXEs\r\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\r\n";
    let p = Wave0Pkg::parse(text).unwrap();
    assert_eq!(p.name, "hello");
    assert_eq!(p.exe, PathBuf::from("hello.exe"));
}

#[test]
fn parse_pkg_keeps_non_exe_files_in_pkg_order() {
    let text = "&EN\n#{\"gui\"},(0xe5d1a001),1,0,0,TYPE=SA\n%{\"Vendor-EN\"}\n:\"Vendor\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"gui.exe\"\t\t-\"!:\\sys\\bin\\gui.exe\"\n\"gui.rsc\"\t\t-\"!:\\resource\\apps\\gui.rsc\"\n\"gui_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc\"\n";
    let p = Wave0Pkg::parse(text).unwrap();
    assert_eq!(p.exe, PathBuf::from("gui.exe"));
    let dests: Vec<&str> = p.files.iter().map(|(_, d)| d.as_str()).collect();
    assert_eq!(
        dests,
        [
            "!:\\resource\\apps\\gui.rsc",
            "!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc"
        ]
    );
}

#[test]
fn parse_wave0_pkg_accepts_verified_reg_rsc_dest() {
    let text = "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n";
    let p = Wave0Pkg::parse(text).unwrap();
    assert_eq!(p.exe, PathBuf::from("hello.exe"));
    assert_eq!(
        p.files,
        [(
            PathBuf::from("hello_reg.rsc"),
            "!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc".to_string()
        )]
    );
}

#[test]
fn run_writes_unsigned_sis() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = dir.path().join("hello.pkg");
    let sis = dir.path().join("hello.sis");
    std::fs::write(dir.path().join("hello.exe"), b"exe").unwrap();
    std::fs::write(
        &pkg,
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n",
    )
    .unwrap();
    Makesis {
        verbose: true,
        pkg,
        sis: sis.clone(),
    }
    .run()
    .unwrap();
    let bytes = std::fs::read(&sis).unwrap();
    assert_eq!(&bytes[..16], &crate::SisUid::new(0xe79e_4cf9).bytes());
}
