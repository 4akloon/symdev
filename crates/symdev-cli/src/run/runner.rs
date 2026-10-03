//! `Runner`: install, launch and follow one app on a device until it ends (spec §6.3–6.5),
//! and for a test binary print its report the way `libtest` does (spec §7).
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use symdev_core::{Error, Result};
use symdev_emulator::control::{CLOSED, ControlClient};
use symdev_emulator::device::{EmulatorProfile, RegistryEntry};
use symdev_emulator::{TestReport, await_report};

use super::{AppExit, CaseLine, ExeTarget, Interrupt, LogTail, TestOutcome, Verdict};
use crate::ld::LinkKind;
use crate::libtest_print::LibtestPrint;

/// How long a running copy of the app gets to go after `app.kill`.
const KILLED: Duration = Duration::from_secs(10);
/// How often the runner looks at Ctrl+C and the log between two exit events.
const TICK: Duration = Duration::from_millis(200);
/// How long a test's report may take to appear once the process ended.
const REPORT: Duration = Duration::from_secs(5);

pub(crate) struct Runner {
    target: ExeTarget,
    device: RegistryEntry,
    client: ControlClient,
}

impl Runner {
    pub fn new(target: ExeTarget, device: RegistryEntry, client: ControlClient) -> Self {
        Self {
            target,
            device,
            client,
        }
    }

    /// Kills a running copy, installs the package over the old one, launches it, and
    /// prints the guest's lines until it exits; its exit is the result.
    ///
    /// A test (`LinkKind::Test`) also has its report: the last one is removed before the
    /// install, the new one is read from the profile's drive E after the exit and printed
    /// as `libtest` would; the status is then 0 exactly when every case passed.
    pub fn run(&mut self, interrupt: &Interrupt, out: &mut dyn Write) -> Result<AppExit> {
        let report = match &self.target.kind {
            LinkKind::Test { .. } => Some(self.result_file()?),
            LinkKind::Main => None,
        };
        if let Some(path) = &report {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    return Err(Error::Other(format!("{}: {e}", path.display())));
                }
                _ => {}
            }
        }
        let exit = match self.follow(interrupt, out, report.as_deref()) {
            Err(e) if e.to_string().starts_with(CLOSED) => {
                AppExit::emulator_closed(&self.device.id)
            }
            other => other?,
        };
        match report {
            Some(path) if exit != AppExit::interrupted() => self.print_tests(&path, &exit, out),
            _ => Ok(exit),
        }
    }

    fn follow(
        &mut self,
        interrupt: &Interrupt,
        out: &mut dyn Write,
        report: Option<&Path>,
    ) -> Result<AppExit> {
        let uid3 = self.target.uid3;
        self.client.subscribe_app_exited()?;
        if self.client.running(uid3)? {
            self.client.kill(uid3)?;
            self.wait_for_exit_of(uid3)?;
        }
        self.client.install(&self.target.sisx)?;
        let mut tail = LogTail::from_end(&self.device.log);
        let pid = self.client.launch(uid3)?;
        loop {
            if interrupt.raised() {
                // The app may have ended meanwhile; the emulator stays either way.
                let _ = self.client.kill(uid3);
                return Ok(AppExit::interrupted());
            }
            let exit = self.client.next_exit(TICK)?;
            for line in tail.poll() {
                writeln!(out, "{line}").map_err(|e| Error::Other(format!("stdout: {e}")))?;
            }
            if let Some(e) = exit.filter(|e| e.pid == pid) {
                return Ok(AppExit::of(&e));
            }
            // A program that reports and keeps running (an Avkon example under
            // `symdev test`) is done once every case of its report is finished.
            if report.is_some_and(finished) {
                let _ = self.client.kill(uid3);
                return Ok(AppExit {
                    code: 0,
                    message: None,
                });
            }
        }
    }

    fn wait_for_exit_of(&mut self, uid3: u32) -> Result<()> {
        let deadline = Instant::now() + KILLED;
        while Instant::now() < deadline {
            if self.client.next_exit(TICK)?.is_some_and(|e| e.uid == uid3) {
                return Ok(());
            }
        }
        Err(Error::Other(format!(
            "0x{uid3:08x} was still running {} s after app.kill on {}",
            KILLED.as_secs(),
            self.device.id
        )))
    }

    /// The report file on the device's profile: `E:\symdev\results\<uid3>.json`.
    fn result_file(&self) -> Result<PathBuf> {
        let root = EmulatorProfile::root_from_env()?;
        let profile = EmulatorProfile::at(&root, &self.device.profile);
        Ok(profile.data().result_file(self.target.uid3))
    }

    fn print_tests(&self, path: &Path, exit: &AppExit, out: &mut dyn Write) -> Result<AppExit> {
        let lines = match await_report(path, REPORT) {
            Ok(report) => TestOutcome::settle(&report, exit),
            Err(_) => vec![CaseLine {
                name: match &self.target.kind {
                    LinkKind::Test { name } => name.clone(),
                    LinkKind::Main => "main".into(),
                },
                verdict: Verdict::Failed,
                detail: format!(
                    "no test report at {}{}",
                    path.display(),
                    exit.message
                        .as_ref()
                        .map(|m| format!(": {m}"))
                        .unwrap_or_default()
                ),
            }],
        };
        let (mut text, passed) = LibtestPrint::lines(&lines);
        // Every case passed, and the program then ended badly (a panic after the last
        // case): the run is not a pass either.
        let ended_badly = passed && exit.code != 0;
        if ended_badly {
            text.push(format!(
                "error: the test program ended with {} after its last test",
                exit.message.as_deref().unwrap_or("a failure")
            ));
        }
        for line in text {
            writeln!(out, "{line}").map_err(|e| Error::Other(format!("stdout: {e}")))?;
        }
        Ok(AppExit {
            code: if passed && !ended_badly { 0 } else { 1 },
            message: None,
        })
    }
}

/// Whether the report at `path` reads, has cases, and has none still pending or running.
fn finished(path: &Path) -> bool {
    TestReport::read(path)
        .is_ok_and(|r| !r.cases.is_empty() && r.cases.iter().all(|c| c.state.is_none()))
}
