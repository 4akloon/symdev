pub fn render_pkg(name: &str, uid3: u32, version: (u32, u32, u32), vendor: &str) -> String {
    let (major, minor, patch) = version;
    format!(
        "&EN\r\n#{{\"{name}\"}},(0x{uid3:08x}),{major},{minor},{patch},TYPE=SA\r\n%{{\"{vendor}\"}}\r\n:\"{vendor}\"\r\n[0x102752AE], 0, 0, 0, {{\"S60ProductID\"}}\r\n\"{name}.exe\"\t\t-\"!:\\sys\\bin\\{name}.exe\"\r\n"
    )
}

#[test]
fn render_pkg_matches_experiment_7_grammar() {
    let s = render_pkg("hello", 0xe79e4cf9, (0, 1, 0), "symdev");
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n"
    );
}
