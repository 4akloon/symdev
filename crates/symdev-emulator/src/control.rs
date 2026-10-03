//! The client of EKA2L1's control server (`--control <socket>`), from its README alone:
//! JSON-RPC 2.0, one message per line, protocol 1.
mod app_exited;
mod control_client;
mod emulator_info;
mod request;

pub use app_exited::{AppExited, ExitType};
pub use control_client::{CALL_TIMEOUT, CLOSED, ControlClient};
pub use emulator_info::{EmulatorInfo, PROTOCOL};
pub(crate) use request::{Param, Request};

#[cfg(test)]
mod tests;
