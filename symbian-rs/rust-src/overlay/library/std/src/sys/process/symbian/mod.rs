//! `std::process` over `RProcess`: spawn, wait and an exit code are real; **stdio
//! redirection is not, and cannot be.**
//!
//! # What a Symbian process inherits, which is one string
//!
//! `RProcess::Create(aFileName, aCommand, aType)` takes an image and **one descriptor**
//! and nothing else. There is no environment (see `sys::env::symbian`), no working
//! directory (the file server keeps one session path, not a per-process one), and no
//! table of file descriptors — Symbian has no such table at all. So
//! [`Command::cwd`] and any change to [`Command::env_mut`] are refused at
//! [`Command::spawn`] rather than silently ignored, which is the failure that would
//! otherwise only show up as a child reading the wrong file.
//!
//! # Why there are no pipes, verified rather than assumed
//!
//! `RPipe` is Symbian 9.4 and later. **It is not in this SDK**: nothing under
//! `epoc32/include` declares it, which was checked rather than taken on trust. There
//! is therefore nothing to redirect a child's output into, and
//! `Command::output()` — and any `Stdio::piped()` — answers `Unsupported`.
//! `Command::spawn()?.wait()` and `Command::status()` are real.
//!
//! # The arguments are re-joined with the rule `sys::args` splits on
//!
//! `std` holds a program and a vector of arguments; Symbian carries one string. The
//! arguments are joined with a single space, which is exactly the rule
//! `sys::args::symbian` splits on, so a `std` parent and a `std` child agree. There is
//! no quoting on either side, because this platform states none — an argument
//! containing a space arrives as two.

mod status;

pub use status::{ExitCode, ExitStatus, ExitStatusError, Process};

use super::env::{CommandEnv, CommandEnvs, CommandResolvedEnvs};
pub use crate::ffi::OsString as EnvKey;
use crate::ffi::{OsStr, OsString};
use crate::path::Path;
use crate::process::StdioPipes;
use crate::sys::fs::File;
use crate::sys::unsupported;
use crate::{fmt, io};

pub struct Command {
    program: OsString,
    args: Vec<OsString>,
    env: CommandEnv,
    cwd: Option<OsString>,
    stdin: Option<Stdio>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
}

#[derive(Debug)]
pub enum Stdio {
    Inherit,
    Null,
    MakePipe,
    ParentStdout,
    ParentStderr,
    #[allow(dead_code)] // This variant exists only for the Debug impl
    InheritFile(File),
}

impl Command {
    pub fn new(program: &OsStr) -> Command {
        Command {
            program: program.to_owned(),
            args: vec![program.to_owned()],
            env: Default::default(),
            cwd: None,
            stdin: None,
            stdout: None,
            stderr: None,
        }
    }

    pub fn arg(&mut self, arg: &OsStr) {
        self.args.push(arg.to_owned());
    }

    pub fn env_mut(&mut self) -> &mut CommandEnv {
        &mut self.env
    }

    pub fn cwd(&mut self, dir: &OsStr) {
        self.cwd = Some(dir.to_owned());
    }

    pub fn stdin(&mut self, stdin: Stdio) {
        self.stdin = Some(stdin);
    }

    pub fn stdout(&mut self, stdout: Stdio) {
        self.stdout = Some(stdout);
    }

    pub fn stderr(&mut self, stderr: Stdio) {
        self.stderr = Some(stderr);
    }

    pub fn get_program(&self) -> &OsStr {
        &self.program
    }

    pub fn get_args(&self) -> CommandArgs<'_> {
        let mut iter = self.args.iter();
        iter.next();
        CommandArgs { iter }
    }

    pub fn get_envs(&self) -> CommandEnvs<'_> {
        self.env.iter()
    }

    pub fn get_env_clear(&self) -> bool {
        self.env.does_clear()
    }

    pub fn get_resolved_envs(&self) -> CommandResolvedEnvs {
        CommandResolvedEnvs::new(self.env.capture())
    }

    pub fn get_current_dir(&self) -> Option<&Path> {
        self.cwd.as_ref().map(|cs| Path::new(cs))
    }

    /// `RProcess::Create` + `Resume`.
    ///
    /// Everything this platform cannot carry into a child is refused here rather than
    /// dropped: a working directory, any environment change, and any stdio that is not
    /// "leave it alone".
    pub fn spawn(&mut self, default: Stdio, _needs_stdin: bool) -> io::Result<(Process, StdioPipes)> {
        if self.cwd.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "RProcess::Create takes no working directory: the file server keeps one \
                 session path per session, not one per process",
            ));
        }
        if !self.env.is_unchanged() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Symbian has no environment variables, so a child cannot be given any \
                 (see std::env on this platform)",
            ));
        }
        for stdio in [&self.stdin, &self.stdout, &self.stderr] {
            check_stdio(stdio.as_ref().unwrap_or(&default))?;
        }
        let process = Process::spawn(&self.program, &self.args[1..])?;
        Ok((process, StdioPipes { stdin: None, stdout: None, stderr: None }))
    }
}

/// The only stdio this platform can honour is "the child is on its own".
fn check_stdio(stdio: &Stdio) -> io::Result<()> {
    match stdio {
        // A Symbian child has no inherited stdio to speak of, and `println!` in every
        // `std` program on this platform goes to the same `E:\symdev\stdout.txt`. So
        // "inherit" is what actually happens, and saying so is not a pretence.
        Stdio::Inherit => Ok(()),
        Stdio::MakePipe => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Stdio::piped() needs RPipe, which is Symbian 9.4 and is not in this SDK: \
             no header under epoc32/include declares it",
        )),
        Stdio::Null | Stdio::ParentStdout | Stdio::ParentStderr | Stdio::InheritFile(_) => {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "a Symbian child inherits one command-line descriptor and no file \
                 handles, so its output cannot be redirected",
            ))
        }
    }
}

/// `Command::output` needs pipes, and there are none. See the module note on `RPipe`.
pub fn output(_cmd: &mut Command) -> io::Result<(ExitStatus, Vec<u8>, Vec<u8>)> {
    unsupported()
}

impl From<ChildPipe> for Stdio {
    fn from(pipe: ChildPipe) -> Stdio {
        pipe.diverge()
    }
}

impl From<io::Stdout> for Stdio {
    fn from(_: io::Stdout) -> Stdio {
        Stdio::ParentStdout
    }
}

impl From<io::Stderr> for Stdio {
    fn from(_: io::Stderr) -> Stdio {
        Stdio::ParentStderr
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Stdio {
        Stdio::InheritFile(file)
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug_command = f.debug_struct("Command");
        debug_command.field("program", &self.program).field("args", &self.args);
        if self.cwd.is_some() {
            debug_command.field("cwd", &self.cwd);
        }
        debug_command.finish()
    }
}

pub struct CommandArgs<'a> {
    iter: crate::slice::Iter<'a, OsString>,
}

impl<'a> Iterator for CommandArgs<'a> {
    type Item = &'a OsStr;

    fn next(&mut self) -> Option<&'a OsStr> {
        self.iter.next().map(|os| &**os)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a> ExactSizeIterator for CommandArgs<'a> {
    fn len(&self) -> usize {
        self.iter.len()
    }

    fn is_empty(&self) -> bool {
        self.iter.is_empty()
    }
}

impl<'a> fmt::Debug for CommandArgs<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter.clone()).finish()
    }
}

pub type ChildPipe = crate::sys::pipe::Pipe;

pub fn read_output(
    out: ChildPipe,
    _stdout: &mut Vec<u8>,
    _err: ChildPipe,
    _stderr: &mut Vec<u8>,
) -> io::Result<()> {
    match out.diverge() {}
}

/// This process's own id, narrowed to 32 bits by `TObjectId::operator TUint()`.
pub fn getpid() -> u32 {
    // SAFETY: a null argument means the current process, which the shim builds with
    // `RProcess()` — `KCurrentProcessHandle`, so there is nothing to open or close.
    // The shim exists because `Id()` returns an 8-byte `TProcessId` by value.
    unsafe { symbian_sys::shim::symrs_process_id(crate::ptr::null()) }
}
