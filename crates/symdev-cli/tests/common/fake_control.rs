//! A fake EKA2L1 for the runner's tests: a control server on a Unix socket that answers each
//! request line from a script and reports every method it got, and a "device" process — a
//! shell script named `eka2l1-fake`, so `/proc/<pid>/comm` says `eka2l1-fake` and
//! `is_eka2l1` accepts it. (A link to `sleep` does not do: this host's `sleep` is a uutils
//! multicall binary that refuses to run under another name.) The script waits on a `read`
//! of a pipe the fake holds, so it has no child to leave behind.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};

/// What the script sends for a request: lines, or `CLOSE` to hang up.
pub const CLOSE: &str = "CLOSE";

pub struct FakeDevice {
    pub env: tempfile::TempDir,
    pub socket: PathBuf,
    pub methods: Receiver<String>,
    sleeper: Child,
    _hold: Option<ChildStdin>,
}

impl FakeDevice {
    /// `script(method, id, line)` answers one request.
    pub fn start(script: impl Fn(&str, &str, &str) -> Vec<String> + Send + 'static) -> Self {
        Self::start_in(tempfile::tempdir().unwrap(), script)
    }

    /// [`Self::start`] with the environment directory made by the caller, who may need its
    /// paths in the script.
    pub fn start_in(
        env: tempfile::TempDir,
        script: impl Fn(&str, &str, &str) -> Vec<String> + Send + 'static,
    ) -> Self {
        let socket = env.path().join("emu.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let (tx, methods) = channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { return };
                let mut w = stream.try_clone().unwrap();
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    let method = field(&line, "\"method\":\"", '"');
                    let id = field(&line, "\"id\":", ',');
                    let _ = tx.send(method.clone());
                    let out = script(&method, &id, &line);
                    if out.iter().any(|l| l == CLOSE) {
                        break;
                    }
                    for l in out {
                        if writeln!(w, "{l}").is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let script = env.path().join("eka2l1-fake");
        std::fs::write(&script, "#!/bin/sh\nread never\n").unwrap();
        let mode = std::os::unix::fs::PermissionsExt::from_mode(0o755);
        std::fs::set_permissions(&script, mode).unwrap();
        let mut sleeper = spawn(&script);
        let hold = sleeper.stdin.take();
        settle(sleeper.id());
        Self {
            env,
            socket,
            methods,
            sleeper,
            _hold: hold,
        }
    }

    pub fn pid(&self) -> u32 {
        self.sleeper.id()
    }

    pub fn alive(&mut self) -> bool {
        self.sleeper.try_wait().unwrap().is_none()
    }

    /// Registers the fake as `emulator-<n>` in the registry under `XDG_RUNTIME_DIR`.
    pub fn register(&self, n: u32) {
        let dir = self.env.path().join("run/symdev/devices");
        std::fs::create_dir_all(&dir).unwrap();
        let entry = format!(
            "id = \"emulator-{n}\"\npid = {}\nprofile = \"rm-469\"\nname = \"Nokia N00 (RM-469)\"\n\
             socket = \"{}\"\nlog = \"{}\"\n",
            self.pid(),
            self.socket.display(),
            self.env.path().join("EKA2L1.log").display()
        );
        std::fs::write(dir.join(format!("emulator-{n}.toml")), entry).unwrap();
    }

    /// `symdev run --exe <image>` in this fake's environment (none of the developer's).
    pub fn runner(&self, image: &Path) -> Command {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin("symdev"));
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("SYMDEV_") {
                cmd.env_remove(&name);
            }
        }
        let e = self.env.path();
        cmd.args(["run", "--exe"])
            .arg(image)
            .env("XDG_RUNTIME_DIR", e.join("run"))
            .env("XDG_DATA_HOME", e.join("data"))
            .env("HOME", e)
            .stdin(std::process::Stdio::null());
        cmd
    }
}

impl Drop for FakeDevice {
    fn drop(&mut self) {
        let _ = self.sleeper.kill();
        let _ = self.sleeper.wait();
    }
}

/// `{"jsonrpc":"2.0","id":<id>,"result":<result>}`.
pub fn answer(id: &str, result: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}")
}

pub const INFO: &str = "{\"name\":\"EKA2L1\",\"version\":\"t\",\"protocol\":1,\"paused\":false,\
    \"device\":{\"manufacturer\":\"Nokia\",\"model\":\"N00\",\"firmware\":\"RM-469\",\"os\":\"epoc93fp2\"}}";

/// `event.app_exited` of pid 7, UID `0xe1234567`.
pub fn exited(exit_type: &str, reason: i64, category: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"event.app_exited\",\"params\":{{\"uid\":3777185127,\
         \"pid\":7,\"name\":\"app\",\"exit_type\":\"{exit_type}\",\"exit_reason\":{reason},\
         \"exit_category\":\"{category}\"}}}}"
    )
}

fn field(line: &str, after: &str, end: char) -> String {
    line.split(after)
        .nth(1)
        .map(|rest| rest.split([end, '}']).next().unwrap_or("").to_string())
        .unwrap_or_default()
}

/// Runs `script`. Another test thread that forks while this one still has the script open
/// for writing makes the exec fail with ETXTBSY, so that is retried.
fn spawn(script: &Path) -> Child {
    for _ in 0..100 {
        match Command::new(script).stdin(Stdio::piped()).spawn() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            other => return other.unwrap(),
        }
    }
    panic!("{} stayed busy", script.display())
}

/// Where the runner looks for the report of UID3 `0xe1234567` on the fake's profile
/// `rm-469`: drive E of the profile under `XDG_DATA_HOME` (`<env>/data`).
pub fn report_path(env: &Path) -> PathBuf {
    env.join("data/symdev/emulators/rm-469/data/drives/e/symdev/results/e1234567.json")
}

/// Waits until the device process's `comm` is its own. `spawn` returns once the exec has
/// closed the parent's close-on-exec pipe, and the kernel sets the new `comm` a moment after
/// that: under load a runner was seen reading the test thread's name there
/// (`a_running_app_i`), and dropping the fake as no EKA2L1.
fn settle(pid: u32) {
    for _ in 0..500 {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        if comm.trim() == "eka2l1-fake" {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("the fake device {pid} never became eka2l1-fake");
}
