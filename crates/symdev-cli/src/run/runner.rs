//! `Runner`: install, launch and follow one app on a device until it ends (spec §6.3–6.5).
use std::io::Write;
use std::time::{Duration, Instant};

use symdev_core::{Error, Result};
use symdev_emulator::control::{CLOSED, ControlClient};
use symdev_emulator::device::RegistryEntry;

use super::{AppExit, ExeTarget, Interrupt, LogTail};

/// How long a running copy of the app gets to go after `app.kill`.
const KILLED: Duration = Duration::from_secs(10);
/// How often the runner looks at Ctrl+C and the log between two exit events.
const TICK: Duration = Duration::from_millis(200);

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
    pub fn run(&mut self, interrupt: &Interrupt, out: &mut dyn Write) -> Result<AppExit> {
        match self.follow(interrupt, out) {
            Err(e) if e.to_string().starts_with(CLOSED) => {
                Ok(AppExit::emulator_closed(&self.device.id))
            }
            other => other,
        }
    }

    fn follow(&mut self, interrupt: &Interrupt, out: &mut dyn Write) -> Result<AppExit> {
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
}
