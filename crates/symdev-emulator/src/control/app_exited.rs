//! `AppExited`: the `event.app_exited` notification (EKA2L1's control README).
use symdev_core::{Error, Result};

use crate::json::Json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitType {
    Kill,
    Terminate,
    Panic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppExited {
    pub uid: u32,
    pub pid: u32,
    pub name: String,
    pub exit_type: ExitType,
    pub reason: i64,
    pub category: String,
}

impl AppExited {
    /// From the notification's `params`.
    pub(crate) fn from_params(params: &Json) -> Result<Self> {
        let bad = |what: &str| Error::Other(format!("event.app_exited without a valid `{what}`"));
        let number = |key: &str| {
            params
                .get(key)
                .and_then(Json::as_i64)
                .ok_or_else(|| bad(key))
        };
        let text = |key: &str| {
            params
                .get(key)
                .and_then(Json::as_str)
                .map(String::from)
                .ok_or_else(|| bad(key))
        };
        let exit_type = match text("exit_type")?.as_str() {
            "kill" => ExitType::Kill,
            "terminate" => ExitType::Terminate,
            "panic" => ExitType::Panic,
            _ => return Err(bad("exit_type")),
        };
        Ok(Self {
            uid: u32::try_from(number("uid")?).map_err(|_| bad("uid"))?,
            pid: u32::try_from(number("pid")?).map_err(|_| bad("pid"))?,
            name: text("name")?,
            exit_type,
            reason: number("exit_reason")?,
            category: text("exit_category")?,
        })
    }
}
