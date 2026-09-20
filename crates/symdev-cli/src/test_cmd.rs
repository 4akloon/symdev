//! `symdev test --emulator`: run the packaged application in EKA2L1 and report what it
//! wrote to `E:\symdev\results\<uid3>.json` (design spec §9, §11).
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use symdev_core::{Error, Result};
use symdev_emulator::{Eka2l1Backend, EmulatorData, TestReport, await_report};

/// How long to wait for the application to write its report.
///
/// EKA2L1 takes tens of seconds to boot the emulated OS, install the SIS and start the
/// process before the application's first line runs, so this is generous; a test that
/// is going to report at all reports long before it.
const TIMEOUT: Duration = Duration::from_secs(180);

pub fn test_project(m: symdev_manifest::Manifest, emulator: bool) -> Result<ExitCode> {
    if !emulator {
        return Err(Error::Other(
            "symdev test needs --emulator: the host-side tests of the SDK crates are \
             `cargo test` in symbian-rs, and there is no other test backend yet"
                .into(),
        ));
    }
    let uid3 = m
        .symbian
        .uid3
        .ok_or_else(|| Error::Other("uid3 required for test (set symbian.uid3)".into()))?;
    let cwd = std::env::current_dir().map_err(|e| Error::Other(e.to_string()))?;
    let sisx = cwd.join("build").join(format!("{}.sisx", m.package.name));
    if !sisx.is_file() {
        return Err(Error::Other(format!(
            "SISX not found: build/{}.sisx (run symdev build && symdev package)",
            m.package.name
        )));
    }
    let result_file = EmulatorData::from_env()?.result_file(uid3);
    clear_stale(&result_file)?;

    let backend = Eka2l1Backend::from_env()?;
    let log = cwd.join("build").join("eka2l1.log");
    let pid_file = cwd.join("build").join("eka2l1.pid");
    if let Some(old) = Eka2l1Backend::previous(&pid_file) {
        eprintln!(
            "warning: EKA2L1 from an earlier run (pid {old}) is still open; two emulators on \
             the same data will fight over the result file"
        );
    }
    let pid = backend.run(&sisx, uid3, &log)?;
    std::fs::write(&pid_file, pid.to_string()).map_err(|e| Error::Other(e.to_string()))?;
    println!(
        "EKA2L1 pid {pid}: running 0x{uid3:08x}, waiting for {}",
        result_file.display()
    );

    let outcome = await_report(&result_file, TIMEOUT);
    // The emulator this command started is this command's to stop, whatever happened;
    // a failure to stop it must not hide the test result, so it is only reported.
    if let Err(e) = Eka2l1Backend::stop(pid) {
        eprintln!("warning: {e}");
    }
    report(&outcome?, &m.package.name)
}

/// Removes a report from an earlier run, so a test that never writes one cannot pass on
/// the last run's file.
fn clear_stale(result_file: &PathBuf) -> Result<()> {
    match std::fs::remove_file(result_file) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Other(format!(
            "remove the previous result {}: {e}",
            result_file.display()
        ))),
    }
}

/// Prints every case and returns the exit code: zero only if the run passed.
fn report(report: &TestReport, name: &str) -> Result<ExitCode> {
    for case in &report.cases {
        let mark = if case.ok { "ok  " } else { "FAIL" };
        if case.detail.is_empty() {
            println!("{mark} {}", case.name);
        } else {
            println!("{mark} {}: {}", case.name, case.detail);
        }
    }
    let (passed, failed) = (report.passed(), report.failed());
    if report.is_pass() {
        println!("{name}: {passed} passed");
        return Ok(ExitCode::SUCCESS);
    }
    if report.cases.is_empty() {
        eprintln!("{name}: the application wrote a report with no cases in it");
    } else {
        eprintln!("{name}: {failed} failed, {passed} passed");
    }
    Ok(ExitCode::from(1))
}
