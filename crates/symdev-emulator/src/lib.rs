//! EKA2L1 as a separate process (GPL-3.0: never linked or vendored): the devices symdev
//! starts and the control protocol.

pub mod control;
pub mod device;
pub(crate) mod json;
mod results;

pub use results::{CaseState, EmulatorData, SCHEMA, TestCase, TestReport, await_report};
