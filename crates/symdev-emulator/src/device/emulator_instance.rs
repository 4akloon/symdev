//! `EmulatorInstance`: an EKA2L1 symdev starts on a profile, and stops (design spec §5–6).
//! It runs in a session of its own (`setsid`), so the terminal's Ctrl+C never reaches it,
//! and it outlives the `cargo run` that started it.
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use symdev_core::{Error, Result};

use super::{DeviceRegistry, EmulatorProfile, RegistryEntry, is_eka2l1};
use crate::control::ControlClient;

/// How long a start may take until `emulator.info` names a device and `apps.list` answers.
const READY: Duration = Duration::from_secs(120);

pub struct EmulatorInstance;

impl EmulatorInstance {
    pub fn argv(eka2l1: &Path, profile_dir: &Path, socket: &Path) -> Vec<OsString> {
        let mut argv: Vec<OsString> = vec!["setsid".into(), eka2l1.into(), "--data-dir".into()];
        argv.extend([profile_dir.into(), "--control".into(), socket.into()]);
        argv
    }

    /// Starts `eka2l1` on `profile` as the next free `emulator-<n>`, waits until it answers,
    /// and registers it. Its output goes to `<profile>/symdev-<id>.out`.
    pub fn start(
        eka2l1: &Path,
        profile: &EmulatorProfile,
        registry: &DeviceRegistry,
    ) -> Result<RegistryEntry> {
        let id = registry.next_id()?;
        let run = registry
            .dir()
            .parent()
            .unwrap_or(registry.dir())
            .to_path_buf();
        std::fs::create_dir_all(&run).map_err(|e| file(&run, e))?;
        let socket = run.join(format!("{id}.sock"));
        let out_path = profile.dir().join(format!("symdev-{id}.out"));
        let out = std::fs::File::create(&out_path).map_err(|e| file(&out_path, e))?;
        let err = out.try_clone().map_err(|e| file(&out_path, e))?;
        let argv = Self::argv(eka2l1, profile.dir(), &socket);
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err)
            .spawn()
            .map_err(|e| Error::Other(format!("start {}: {e}", eka2l1.display())))?;
        let pid = child.id();
        let deadline = Instant::now() + READY;
        let name = loop {
            if let Ok(Some(status)) = child.try_wait() {
                let said = std::fs::read_to_string(&out_path).unwrap_or_default();
                return Err(Error::Other(format!(
                    "{} exited ({status}) before it answered on {}: {}",
                    eka2l1.display(),
                    socket.display(),
                    said.lines().rev().take(5).collect::<Vec<_>>().join(" | ")
                )));
            }
            let ready = ControlClient::connect(&socket)
                .and_then(|mut c| c.info().and_then(|info| c.running(0).map(|_| info.name)));
            if let Ok(name) = ready {
                break name;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                return Err(Error::Other(format!(
                    "{} did not answer on {} within {} s; symdev killed it (pid {pid}); its \
                     output is in {}",
                    eka2l1.display(),
                    socket.display(),
                    READY.as_secs(),
                    out_path.display()
                )));
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        let entry = RegistryEntry {
            id,
            pid,
            profile: profile.name().to_string(),
            name,
            socket,
            log: profile.log_file(),
        };
        registry.add(&entry)?;
        Ok(entry)
    }

    /// Ends a registered instance with `kill -9` (EKA2L1 ignores SIGTERM), only while its
    /// PID is still an EKA2L1, and removes its entry either way.
    pub fn stop(entry: &RegistryEntry, registry: &DeviceRegistry) -> Result<()> {
        if is_eka2l1(entry.pid) {
            let status = Command::new("kill")
                .args(["-9", &entry.pid.to_string()])
                .status()
                .map_err(|e| Error::Other(format!("kill -9 {}: {e}", entry.pid)))?;
            if !status.success() {
                return Err(Error::Other(format!(
                    "kill -9 {} failed: {status}",
                    entry.pid
                )));
            }
        }
        registry.remove(&entry.id)
    }

    /// Whether `eka2l1 --help` lists `--control`. Observed (experiment 114 §2): the help
    /// is printed first and the process then does not exit, and any run without
    /// `--data-dir` uses (and rotates the log of) the default data folder. So it runs with
    /// `--data-dir` and `HOME`/`XDG_*` in a scratch folder, is read for 15 s at most, and is
    /// killed.
    pub fn has_control(eka2l1: &Path) -> Result<bool> {
        let scratch =
            std::env::temp_dir().join(format!("symdev-eka2l1-help-{}", std::process::id()));
        std::fs::create_dir_all(&scratch).map_err(|e| file(&scratch, e))?;
        let mut child = Command::new(eka2l1)
            .arg("--data-dir")
            .arg(scratch.join("data"))
            .arg("--help")
            .env("HOME", &scratch)
            .env("XDG_DATA_HOME", scratch.join("share"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Other(format!("run {} --help: {e}", eka2l1.display())))?;
        let (tx, rx) = std::sync::mpsc::channel();
        if let Some(stdout) = child.stdout.take() {
            std::thread::spawn(move || {
                let found = BufReader::new(stdout)
                    .lines()
                    .map_while(std::result::Result::ok)
                    .any(|l| l.trim_start().starts_with("--control"));
                let _ = tx.send(found);
            });
        }
        let found = rx.recv_timeout(Duration::from_secs(15)).unwrap_or(false);
        let _ = child.kill();
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&scratch);
        Ok(found)
    }
}

fn file(path: &Path, e: std::io::Error) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}
