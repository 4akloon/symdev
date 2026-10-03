use std::path::Path;
use std::time::{Duration, SystemTime};

use super::{AppExit, ExeTarget};
use crate::ld::{LinkKind, LinkRecord};
use symdev_emulator::control::{AppExited, ExitType};

fn image(dir: &Path, rel: &str, uid3: u32) -> std::path::PathBuf {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut h = vec![0u8; 0x9c];
    h[0..4].copy_from_slice(&0x1000_007a_u32.to_le_bytes());
    h[8..12].copy_from_slice(&uid3.to_le_bytes());
    std::fs::write(&p, h).unwrap();
    std::fs::write(format!("{}.sisx", p.display()), b"sisx").unwrap();
    LinkRecord {
        kind: LinkKind::Main,
    }
    .write(Path::new(&format!("{}.symdev.toml", p.display())))
    .unwrap();
    p
}

#[test]
fn a_relative_exe_is_resolved_against_the_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    let abs = image(
        dir.path(),
        "app/build/cargo/arm-symbian-e32/debug/app",
        0xe1234567,
    );
    // cargo runs the runner from an existing directory, here a subdirectory of the project.
    std::fs::create_dir_all(dir.path().join("app/src")).unwrap();
    let t = ExeTarget::of(
        Path::new("../build/cargo/arm-symbian-e32/debug/app"),
        &dir.path().join("app/src"),
    )
    .unwrap();
    assert_eq!(t.image.canonicalize().unwrap(), abs.canonicalize().unwrap());
    assert_eq!((t.uid3, t.kind), (0xe1234567, LinkKind::Main));
}

#[test]
fn a_sisx_older_than_its_image_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let img = image(dir.path(), "out/app", 1);
    let old = SystemTime::now() - Duration::from_secs(60);
    std::fs::File::options()
        .write(true)
        .open(format!("{}.sisx", img.display()))
        .unwrap()
        .set_modified(old)
        .unwrap();
    let e = ExeTarget::of(&img, dir.path()).unwrap_err().to_string();
    assert!(e.contains("older than") && e.contains("cargo build"), "{e}");
}

#[test]
fn an_image_without_a_sisx_was_not_linked_by_symdev_ld() {
    let dir = tempfile::tempdir().unwrap();
    let img = image(dir.path(), "out/app", 1);
    std::fs::remove_file(format!("{}.sisx", img.display())).unwrap();
    let e = ExeTarget::of(&img, dir.path()).unwrap_err().to_string();
    assert!(
        e.contains("symdev-ld") && e.contains(".cargo/config.toml"),
        "{e}"
    );
}

#[test]
fn exits_map_as_the_spec_table_says() {
    let ev = |t, reason, cat: &str| AppExited {
        uid: 1,
        pid: 2,
        name: "a".into(),
        exit_type: t,
        reason,
        category: cat.into(),
    };
    assert_eq!(
        AppExit::of(&ev(ExitType::Kill, 0, "None")),
        AppExit {
            code: 0,
            message: None
        }
    );
    assert_eq!(
        AppExit::of(&ev(ExitType::Panic, 3, "RUST")),
        AppExit {
            code: 101,
            message: Some("panicked: RUST 3".into())
        }
    );
    assert_eq!(
        AppExit::of(&ev(ExitType::Kill, 0, "Kill")),
        AppExit {
            code: 1,
            message: Some("kill: Kill 0".into())
        }
    );
    assert_eq!(AppExit::of(&ev(ExitType::Terminate, -1, "None")).code, 1);
    assert_eq!(
        AppExit::interrupted(),
        AppExit {
            code: 130,
            message: None
        }
    );
    let id = symdev_emulator::device::DeviceId::parse("emulator-1").unwrap();
    assert_eq!(
        AppExit::emulator_closed(&id).message.as_deref(),
        Some("emulator-1 was closed")
    );
}

#[test]
fn arguments_after_the_exe_are_refused_until_observed() {
    let e = super::refuse_arguments(&["a".into()])
        .unwrap_err()
        .to_string();
    assert!(e.contains("not observed"), "{e}");
}

#[test]
fn the_log_tail_gives_the_guest_lines_written_after_it_started() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("EKA2L1.log");
    std::fs::write(&log, "T k.cpp:1 [Emulated.Stdout]: before\n").unwrap();
    let mut tail = super::LogTail::from_end(&log);
    let mut f = std::fs::OpenOptions::new().append(true).open(&log).unwrap();
    use std::io::Write as _;
    write!(f, "T s.cpp:3 [Emulated.Stdout]: hello there\nI a.cpp:9 [Kernel]: other\nT s.cpp:3 [Emulated.Stdout]: half").unwrap();
    assert_eq!(tail.poll(), vec!["hello there".to_string()]);
    writeln!(f, " a line").unwrap();
    assert_eq!(tail.poll(), vec!["half a line".to_string()]);
    std::fs::write(&log, "T s.cpp:3 [Emulated.Stdout]: after a restart\n").unwrap();
    assert_eq!(tail.poll(), vec!["after a restart".to_string()]);
}
