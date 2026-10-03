//! `AppExit`: how the app ended, as the runner's exit status and message (spec §6.5).
use symdev_emulator::control::{AppExited, ExitType};
use symdev_emulator::device::DeviceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppExit {
    pub code: u8,
    pub message: Option<String>,
}

impl AppExit {
    pub fn of(e: &AppExited) -> Self {
        match e.exit_type {
            // An app that ended by itself (EKA2L1's control README).
            ExitType::Kill if e.reason == 0 && e.category == "None" => Self {
                code: 0,
                message: None,
            },
            ExitType::Panic => Self {
                code: 101,
                message: Some(format!("panicked: {} {}", e.category, e.reason)),
            },
            ExitType::Kill | ExitType::Terminate => Self {
                code: 1,
                message: Some(format!(
                    "{}: {} {}",
                    if e.exit_type == ExitType::Kill {
                        "kill"
                    } else {
                        "terminate"
                    },
                    e.category,
                    e.reason
                )),
            },
        }
    }

    /// Ctrl+C: the runner killed the app; the emulator stays.
    pub fn interrupted() -> Self {
        Self {
            code: 130,
            message: None,
        }
    }

    pub fn emulator_closed(id: &DeviceId) -> Self {
        Self {
            code: 1,
            message: Some(format!("{id} was closed")),
        }
    }
}
