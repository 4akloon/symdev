//! `EmulatorInfo`: what `emulator.info` says about the running device.
use symdev_core::{Error, Result};

use crate::json::Json;

/// The protocol symdev speaks (EKA2L1's control README).
pub const PROTOCOL: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmulatorInfo {
    /// `"<manufacturer> <model> (<firmware>)"` as reported, e.g. `Nokia N00 (RM-469)`.
    pub name: String,
}

impl EmulatorInfo {
    /// From the result of `emulator.info`. Another protocol is refused, and no booted
    /// device yet is an error the caller may retry.
    pub(crate) fn from_result(result: &Json) -> Result<Self> {
        let protocol = result.get("protocol").and_then(Json::as_i64);
        if protocol != Some(PROTOCOL) {
            return Err(Error::Other(format!(
                "this EKA2L1 speaks control protocol {}, symdev speaks {PROTOCOL}: set \
                 SYMDEV_EKA2L1 to an EKA2L1 with the control server of protocol {PROTOCOL} \
                 (EKA2L1#770–#772, our fork's symdev branch)",
                protocol.map_or("(none)".to_string(), |p| p.to_string())
            )));
        }
        let device = result
            .get("device")
            .filter(|d| **d != Json::Null)
            .ok_or_else(|| Error::Other("the emulator has not booted a device yet".into()))?;
        let field = |key: &str| device.get(key).and_then(Json::as_str).unwrap_or("?");
        Ok(Self {
            name: format!(
                "{} {} ({})",
                field("manufacturer"),
                field("model"),
                field("firmware")
            ),
        })
    }
}
