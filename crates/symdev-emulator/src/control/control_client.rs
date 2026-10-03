//! `ControlClient`: JSON-RPC 2.0 to one EKA2L1 started with `--control <socket>`, one
//! message per line (EKA2L1's control README, protocol 1). Written from the README alone.
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use symdev_core::{Error, Result};

use super::{AppExited, EmulatorInfo, Param, Request};
use crate::json::Json;

/// The text every "the emulator went away" error starts with.
pub const CLOSED: &str = "the emulator closed its control connection";

pub struct ControlClient {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next_id: u64,
    /// Notifications that arrived while a call waited for its answer.
    exits: VecDeque<AppExited>,
    /// A line read only in part when a read timed out.
    pending: String,
}

impl ControlClient {
    pub fn connect(socket: &Path) -> Result<Self> {
        let stream = UnixStream::connect(socket).map_err(|e| {
            Error::Other(format!(
                "connect to the emulator at {}: {e}",
                socket.display()
            ))
        })?;
        let writer = stream.try_clone().map_err(io)?;
        Ok(Self {
            reader: BufReader::new(stream),
            writer,
            next_id: 0,
            exits: VecDeque::new(),
            pending: String::new(),
        })
    }

    pub fn info(&mut self) -> Result<EmulatorInfo> {
        EmulatorInfo::from_result(&self.call("emulator.info", &[])?)
    }

    /// Whether a process of the app `uid3` runs now (`apps.list`'s `running`).
    pub fn running(&mut self, uid3: u32) -> Result<bool> {
        let list = self.call("apps.list", &[])?;
        let apps = list.get("apps").and_then(Json::as_array).unwrap_or(&[]);
        Ok(apps.iter().any(|a| {
            a.get("uid").and_then(Json::as_i64) == Some(i64::from(uid3))
                && a.get("running").and_then(Json::as_bool) == Some(true)
        }))
    }

    pub fn install(&mut self, sisx: &Path) -> Result<()> {
        let path = sisx.display().to_string();
        self.call("package.install", &[("path", Param::Str(&path))])
            .map(|_| ())
    }

    /// Starts the app; its process id.
    pub fn launch(&mut self, uid3: u32) -> Result<u32> {
        let r = self.call("app.launch", &[("uid", Param::Uid(uid3))])?;
        Self::count(&r, "pid", "app.launch")
    }

    /// Kills every process of the app; how many.
    pub fn kill(&mut self, uid3: u32) -> Result<u32> {
        let r = self.call("app.kill", &[("uid", Param::Uid(uid3))])?;
        Self::count(&r, "killed", "app.kill")
    }

    pub fn subscribe_app_exited(&mut self) -> Result<()> {
        self.call(
            "events.subscribe",
            &[("events", Param::Names(&["app_exited"]))],
        )
        .map(|_| ())
    }

    /// The next `event.app_exited`, waiting up to `timeout`; `None` when none came.
    pub fn next_exit(&mut self, timeout: Duration) -> Result<Option<AppExited>> {
        if let Some(exit) = self.exits.pop_front() {
            return Ok(Some(exit));
        }
        let wait = timeout.max(Duration::from_millis(1));
        self.reader
            .get_ref()
            .set_read_timeout(Some(wait))
            .map_err(io)?;
        let line = self.read_line();
        self.reader.get_ref().set_read_timeout(None).map_err(io)?;
        match line? {
            None => Ok(None),
            Some(line) => {
                self.take(&line)?;
                Ok(self.exits.pop_front())
            }
        }
    }

    /// One call: its `result`, or an error naming the method, the code and the message.
    fn call(&mut self, method: &str, params: &[(&str, Param)]) -> Result<Json> {
        self.next_id += 1;
        let id = self.next_id;
        let mut line = Request::line(id, method, params);
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| closed_or(e, method))?;
        loop {
            let Some(text) = self.read_line()? else {
                continue;
            };
            let Some(message) = self.take(&text)? else {
                continue;
            };
            if message.get("id").and_then(Json::as_i64) != i64::try_from(id).ok() {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(Error::Other(format!(
                    "the emulator refused {method}: error {}: {}",
                    error.get("code").and_then(Json::as_i64).unwrap_or(0),
                    error.get("message").and_then(Json::as_str).unwrap_or("")
                )));
            }
            return Ok(message.get("result").cloned().unwrap_or(Json::Null));
        }
    }

    /// Queues an `event.app_exited`; any other message is handed back.
    fn take(&mut self, line: &str) -> Result<Option<Json>> {
        let message = Json::parse(line)?;
        if message.get("method").and_then(Json::as_str) == Some("event.app_exited") {
            let params = message.get("params").cloned().unwrap_or(Json::Null);
            self.exits.push_back(AppExited::from_params(&params)?);
            return Ok(None);
        }
        Ok(Some(message))
    }

    /// One whole line; `None` when a read timed out first (the part read is kept).
    fn read_line(&mut self) -> Result<Option<String>> {
        match self.reader.read_line(&mut self.pending) {
            Ok(0) => Err(Error::Other(format!(
                "{CLOSED}: it exited, or its window was closed"
            ))),
            Ok(_) if self.pending.ends_with('\n') => Ok(Some(std::mem::take(&mut self.pending))),
            Ok(_) => Ok(None),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => Ok(None),
            Err(e) => Err(closed_or(e, "reading")),
        }
    }

    fn count(result: &Json, key: &str, method: &str) -> Result<u32> {
        result
            .get(key)
            .and_then(Json::as_i64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| Error::Other(format!("{method} answered without `{key}`")))
    }
}

fn closed_or(e: std::io::Error, what: &str) -> Error {
    match e.kind() {
        ErrorKind::BrokenPipe | ErrorKind::ConnectionReset => Error::Other(format!(
            "{CLOSED} ({what}): it exited, or its window was closed"
        )),
        _ => Error::Other(format!("control connection, {what}: {e}")),
    }
}

fn io(e: std::io::Error) -> Error {
    Error::Other(format!("control connection: {e}"))
}
