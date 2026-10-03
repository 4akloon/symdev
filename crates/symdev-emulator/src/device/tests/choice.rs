use std::path::PathBuf;

use crate::device::{Choice, DeviceChoice, DeviceId, DevicePrompt, Offer, RegistryEntry};

fn emu(n: u32, profile: &str) -> RegistryEntry {
    RegistryEntry {
        id: DeviceId::parse(&format!("emulator-{n}")).unwrap(),
        pid: 100 + n,
        profile: profile.into(),
        name: "Nokia E52 (RM-469)".into(),
        socket: PathBuf::from(format!("/run/s{n}")),
        log: PathBuf::from("/l"),
    }
}
fn id(s: &str) -> DeviceId {
    DeviceId::parse(s).unwrap()
}
fn choose(
    requested: Option<&str>,
    running: Vec<RegistryEntry>,
    profiles: &[&str],
    terminal: bool,
) -> Choice {
    DeviceChoice {
        requested: requested.map(String::from),
        running,
        profiles: profiles.iter().map(|s| s.to_string()).collect(),
        terminal,
    }
    .decide()
}

#[test]
fn the_rules_of_spec_section_5() {
    // 1. SYMDEV_DEVICE wins: a running id, a running profile, a profile to start.
    assert_eq!(
        choose(
            Some("emulator-2"),
            vec![emu(1, "rm-469"), emu(2, "rm-469")],
            &["rm-469"],
            false
        ),
        Choice::Use(id("emulator-2"))
    );
    assert_eq!(
        choose(Some("rm-469"), vec![emu(3, "rm-469")], &["rm-469"], false),
        Choice::Use(id("emulator-3"))
    );
    assert_eq!(
        choose(Some("rm-469"), vec![], &["rm-469"], false),
        Choice::Start("rm-469".into())
    );
    // 2. exactly one running → it, even with several profiles.
    assert_eq!(
        choose(None, vec![emu(1, "a")], &["a", "b"], false),
        Choice::Use(id("emulator-1"))
    );
    // 3. none running, one profile → start it.
    assert_eq!(
        choose(None, vec![], &["rm-469"], false),
        Choice::Start("rm-469".into())
    );
    // 4. otherwise ask on a terminal …
    assert!(
        matches!(choose(None, vec![emu(1, "a"), emu(2, "a")], &["a"], true), Choice::Ask(o) if o.len() == 3)
    );
    assert!(matches!(choose(None, vec![], &["a", "b"], true), Choice::Ask(o) if o.len() == 2));
}

#[test]
fn several_devices_and_no_terminal_list_the_ids_and_name_symdev_device() {
    let Choice::Refuse(e) = choose(None, vec![emu(1, "a"), emu(2, "a")], &["a"], false) else {
        panic!()
    };
    assert!(
        e.contains("emulator-1") && e.contains("emulator-2") && e.contains("SYMDEV_DEVICE"),
        "{e}"
    );
}

#[test]
fn an_unknown_symdev_device_and_no_profile_at_all_are_refused() {
    let Choice::Refuse(e) = choose(Some("emulator-9"), vec![emu(1, "a")], &["a"], true) else {
        panic!()
    };
    assert!(
        e.contains("emulator-9") && e.contains("emulator-1") && e.contains("a"),
        "{e}"
    );
    let Choice::Refuse(e) = choose(None, vec![], &[], true) else {
        panic!()
    };
    assert!(e.contains("firmware"), "{e}");
}

#[test]
fn the_prompt_numbers_offers_and_takes_a_digit_or_q() {
    let offers = vec![
        Offer::Running {
            id: id("emulator-1"),
            name: "Nokia E52 (RM-469)".into(),
        },
        Offer::Profile("rm-469".into()),
    ];
    assert_eq!(
        DevicePrompt::lines(&offers),
        vec![
            "[1]: Nokia E52 (RM-469) (emulator-1)".to_string(),
            "[2]: rm-469 (start a new emulator)".to_string()
        ]
    );
    assert_eq!(
        DevicePrompt::answer("2\n", &offers).unwrap(),
        Some(offers[1].clone())
    );
    assert_eq!(DevicePrompt::answer("q", &offers).unwrap(), None);
    assert!(
        DevicePrompt::answer("3", &offers).is_err() && DevicePrompt::answer("x", &offers).is_err()
    );
}

#[test]
fn an_entry_whose_pid_is_not_eka2l1_is_dropped_not_killed() {
    let dir = tempfile::tempdir().unwrap();
    let reg = crate::device::DeviceRegistry::at(dir.path().to_path_buf());
    let mut mine = emu(1, "a");
    mine.pid = std::process::id(); // this test process: alive, and not an EKA2L1
    reg.add(&mine).unwrap();
    let live = reg.live(crate::device::is_eka2l1, |_| true).unwrap();
    assert!(live.is_empty());
    assert!(!dir.path().join("emulator-1.toml").exists());
    // still here, so nothing signalled us
    assert!(std::path::Path::new(&format!("/proc/{}", std::process::id())).exists());
}

#[test]
fn the_next_id_is_the_lowest_free_one() {
    let dir = tempfile::tempdir().unwrap();
    let reg = crate::device::DeviceRegistry::at(dir.path().to_path_buf());
    reg.add(&emu(1, "a")).unwrap();
    reg.add(&emu(3, "a")).unwrap();
    assert_eq!(reg.next_id().unwrap(), id("emulator-2"));
}

#[test]
fn a_device_id_is_emulator_and_a_positive_number() {
    assert_eq!(id("emulator-12").to_string(), "emulator-12");
    for not in [
        "emulator-0",
        "emulator-01",
        "emulator-",
        "emulator--1",
        "rm-469",
        "emulator-1x",
    ] {
        assert_eq!(DeviceId::parse(not), None, "{not}");
    }
}

#[test]
fn an_entry_reads_back_what_was_written() {
    let mut e = emu(4, "rm-469");
    e.name = "Nokia N00 (RM-469) \"quoted\"".into();
    let back = RegistryEntry::from_toml(&e.to_toml(), std::path::Path::new("x.toml")).unwrap();
    assert_eq!(back, e);
}

#[test]
fn an_eka2l1_that_does_not_answer_stays_registered_but_is_not_live() {
    let dir = tempfile::tempdir().unwrap();
    let reg = crate::device::DeviceRegistry::at(dir.path().to_path_buf());
    reg.add(&emu(1, "a")).unwrap();
    assert!(reg.live(|_| true, |_| false).unwrap().is_empty());
    assert!(dir.path().join("emulator-1.toml").exists());
    let kept = reg.registered(|_| true).unwrap();
    assert_eq!(
        kept.iter().map(|e| e.id).collect::<Vec<_>>(),
        [id("emulator-1")]
    );
    assert!(reg.registered(|_| false).unwrap().is_empty());
    assert!(!dir.path().join("emulator-1.toml").exists());
}
