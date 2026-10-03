//! EKA2L1 as a separate process (GPL-3.0: never linked or vendored).

use std::path::PathBuf;

use symdev_core::{Error, Result};

pub mod control;
pub mod device;
pub(crate) mod json;
mod results;

pub use results::{CaseState, EmulatorData, SCHEMA, TestCase, TestReport, await_report};

/// The EKA2L1 to start (`SYMDEV_EKA2L1`): a binary or a wrapper. symdev starts it with
/// `--data-dir <profile> --control <socket>` (`device::EmulatorInstance`).
pub struct Eka2l1Backend {
    pub eka2l1: PathBuf,
}

impl Eka2l1Backend {
    pub fn from_env() -> Result<Self> {
        match std::env::var_os("SYMDEV_EKA2L1") {
            Some(v) if !v.is_empty() => Ok(Self {
                eka2l1: PathBuf::from(v),
            }),
            _ => Err(Error::Other(
                "missing emulator: SYMDEV_EKA2L1 (path to eka2l1_qt or a wrapper)".into(),
            )),
        }
    }
}
