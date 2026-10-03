//! `symdev run --exe <image>`, cargo's runner, against a fake EKA2L1 (design spec §6).
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::Duration;

mod common;
use common::fake_control::{CLOSE, FakeDevice, INFO, answer, exited, report_path};

const UID3: u32 = 0xe123_4567;

/// An image as symdev-ld leaves it: the E32 header's UID1 and UID3, `<image>.sisx`, and the
/// link record of the main binary.
fn image(dir: &Path) -> PathBuf {
    let p = dir.join("out/app");
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    let mut h = vec![0u8; 0x9c];
    h[0..4].copy_from_slice(&0x1000_007a_u32.to_le_bytes());
    h[8..12].copy_from_slice(&UID3.to_le_bytes());
    std::fs::write(&p, h).unwrap();
    std::fs::write(dir.join("out/app.sisx"), b"sisx").unwrap();
    std::fs::write(dir.join("out/app.symdev.toml"), "kind = \"main\"\n").unwrap();
    p
}

/// The fake's answers for a run: `then` is what follows the answer to `app.launch`.
fn device(running: bool, then: Vec<String>) -> FakeDevice {
    device_writing(tempfile::tempdir().unwrap(), running, then, None)
}

/// [`device`] in `env`, writing `report` as the device's test report just before it sends
/// what follows `app.launch`.
fn device_writing(
    env: tempfile::TempDir,
    running: bool,
    then: Vec<String>,
    report: Option<String>,
) -> FakeDevice {
    let path = report_path(env.path());
    FakeDevice::start_in(env, move |method, id, _| match method {
        "emulator.info" => vec![answer(id, INFO)],
        "events.subscribe" => vec![answer(id, "{\"events\":[\"app_exited\"]}")],
        "apps.list" => vec![answer(
            id,
            &format!("{{\"apps\":[{{\"uid\":{UID3},\"running\":{running}}}]}}"),
        )],
        "app.kill" => vec![answer(id, "{\"killed\":1}"), exited("kill", 0, "Kill")],
        "package.install" => vec![answer(id, "{}")],
        "app.launch" => {
            if let Some(report) = &report {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, report).unwrap();
            }
            let mut out = vec![answer(id, "{\"pid\":7}")];
            out.extend(then.clone());
            out
        }
        _ => vec![answer(id, "{}")],
    })
}

fn methods(rx: &Receiver<String>) -> Vec<String> {
    rx.try_iter().filter(|m| m != "emulator.info").collect()
}

fn run(fake: &FakeDevice, image: &Path) -> (i32, String) {
    let mut cmd = fake.runner(image);
    cmd.env("SYMDEV_DEVICE", "emulator-1");
    let out = cmd.output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn a_normal_exit_is_status_0() {
    let fake = device(false, vec![exited("kill", 0, "None")]);
    fake.register(1);
    let (code, err) = run(&fake, &image(fake.env.path()));
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        methods(&fake.methods),
        [
            "events.subscribe",
            "apps.list",
            "package.install",
            "app.launch"
        ]
    );
}

#[test]
fn a_panic_is_101_and_prints_its_category_and_reason() {
    let fake = device(false, vec![exited("panic", 3, "RUST")]);
    fake.register(1);
    let (code, err) = run(&fake, &image(fake.env.path()));
    assert_eq!(code, 101, "{err}");
    assert!(err.contains("panicked: RUST 3"), "{err}");
}

#[test]
fn a_running_app_is_killed_before_the_install() {
    let fake = device(true, vec![exited("kill", 0, "None")]);
    fake.register(1);
    let (code, err) = run(&fake, &image(fake.env.path()));
    assert_eq!(code, 0, "{err}");
    let m = methods(&fake.methods);
    let at = |name: &str| m.iter().position(|x| x == name).unwrap();
    assert!(at("app.kill") < at("package.install"), "{m:?}");
}

#[test]
fn ctrl_c_kills_the_app_and_leaves_the_emulator() {
    let mut fake = device(false, Vec::new());
    fake.register(1);
    let mut child = fake.runner(&image(fake.env.path()));
    child.env("SYMDEV_DEVICE", "emulator-1");
    let mut child = child.stderr(std::process::Stdio::piped()).spawn().unwrap();
    let mut seen = Vec::new();
    while !seen.iter().any(|m| m == "app.launch") {
        seen.push(fake.methods.recv_timeout(Duration::from_secs(20)).unwrap());
    }
    std::thread::sleep(Duration::from_millis(300));
    let pid = child.id().to_string();
    assert!(
        std::process::Command::new("kill")
            .args(["-INT", &pid])
            .status()
            .unwrap()
            .success()
    );
    let status = child.wait().unwrap();
    let err = std::io::read_to_string(child.stderr.take().unwrap()).unwrap_or_default();
    assert_eq!(status.code(), Some(130), "{status:?} {err}");
    assert!(fake.methods.try_iter().any(|m| m == "app.kill"));
    assert!(fake.alive());
}

#[test]
fn a_closed_emulator_is_reported() {
    let fake = device(false, vec![CLOSE.to_string()]);
    fake.register(1);
    let (code, err) = run(&fake, &image(fake.env.path()));
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("emulator-1 was closed"), "{err}");
}

#[test]
fn several_devices_and_no_terminal_is_an_error() {
    let fake = device(false, Vec::new());
    fake.register(1);
    fake.register(2);
    let out = fake.runner(&image(fake.env.path())).output().unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(
        err.contains("emulator-1") && err.contains("emulator-2") && err.contains("SYMDEV_DEVICE"),
        "{err}"
    );
}

/// The image of a `harness = false` test, as symdev-ld records it.
fn test_image(dir: &Path) -> PathBuf {
    let p = image(dir);
    std::fs::write(
        dir.join("out/app.symdev.toml"),
        "kind = \"test\"\nname = \"smoke\"\n",
    )
    .unwrap();
    p
}

fn report(cases: &str) -> String {
    format!(
        "{{\"schema\":1,\"app\":\"smoke\",\"uid3\":\"0xe1234567\",\"passed\":0,\"failed\":0,\"cases\":[{cases}]}}"
    )
}

fn run_test(fake: &FakeDevice) -> (i32, String, String) {
    let mut cmd = fake.runner(&test_image(fake.env.path()));
    cmd.env("SYMDEV_DEVICE", "emulator-1");
    let out = cmd.output().unwrap();
    let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
    (
        out.status.code().unwrap_or(-1),
        text(&out.stdout),
        text(&out.stderr),
    )
}

#[test]
fn cargo_test_prints_libtest_lines_and_exits_0() {
    let cases = report("{\"name\":\"a\",\"ok\":true},{\"name\":\"b\",\"ok\":true}");
    let fake = device_writing(
        tempfile::tempdir().unwrap(),
        false,
        vec![exited("kill", 0, "None")],
        Some(cases),
    );
    fake.register(1);
    let (code, out, err) = run_test(&fake);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("test a ... ok") && out.contains("test result: ok. 2 passed; 0 failed"),
        "{out}"
    );
}

#[test]
fn a_failing_case_exits_non_zero() {
    let cases = report("{\"name\":\"x\",\"ok\":false,\"detail\":\"2 + 2 is 4\"}");
    let fake = device_writing(
        tempfile::tempdir().unwrap(),
        false,
        vec![exited("kill", 0, "None")],
        Some(cases),
    );
    fake.register(1);
    let (code, out, err) = run_test(&fake);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        out.contains("test x ... FAILED") && out.contains("failures:\n    x: 2 + 2 is 4"),
        "{out}"
    );
}

#[test]
fn a_report_from_an_earlier_run_is_not_read() {
    let env = tempfile::tempdir().unwrap();
    let stale = report_path(env.path());
    std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
    std::fs::write(&stale, report("{\"name\":\"old\",\"ok\":true}")).unwrap();
    let fake = device_writing(env, false, vec![exited("kill", 0, "None")], None);
    fake.register(1);
    let (code, out, err) = run_test(&fake);
    assert_eq!(code, 1, "{out}{err}");
    assert!(
        !stale.exists() && out.contains("no test report"),
        "{out}{err}"
    );
}

/// An emulator that stops answering holds the runner in a call; a second Ctrl+C ends it at
/// once with 130, as cargo's own runner convention has it.
#[test]
fn a_second_ctrl_c_ends_a_runner_waiting_on_the_emulator() {
    let fake = FakeDevice::start(|method, id, _| match method {
        "emulator.info" => vec![answer(id, INFO)],
        "events.subscribe" => vec![answer(id, "{\"events\":[\"app_exited\"]}")],
        "apps.list" => vec![answer(
            id,
            &format!("{{\"apps\":[{{\"uid\":{UID3},\"running\":false}}]}}"),
        )],
        _ => Vec::new(), // package.install never answers
    });
    fake.register(1);
    let mut cmd = fake.runner(&image(fake.env.path()));
    cmd.env("SYMDEV_DEVICE", "emulator-1");
    let mut child = cmd.stderr(std::process::Stdio::piped()).spawn().unwrap();
    let mut seen = Vec::new();
    while !seen.iter().any(|m| m == "package.install") {
        seen.push(fake.methods.recv_timeout(Duration::from_secs(20)).unwrap());
    }
    let pid = child.id().to_string();
    for _ in 0..2 {
        std::thread::sleep(Duration::from_millis(300));
        let _ = std::process::Command::new("kill")
            .args(["-INT", &pid])
            .status();
    }
    let started = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break Some(status);
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(status.and_then(|s| s.code()), Some(130));
}
