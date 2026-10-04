//! Devices `cargo run` and `cargo test` run on (design spec §5): emulator profiles, the
//! registry of the emulators symdev started, and the choice among them.
mod device_choice;
mod device_id;
mod device_prompt;
mod device_registry;
mod eka2l1;
mod emulator_instance;
mod emulator_profile;
mod firmware;
mod profile_check;
mod registry_entry;

pub use device_choice::{Choice, DeviceChoice, Offer};
pub use device_id::DeviceId;
pub use device_prompt::DevicePrompt;
pub use device_registry::{DeviceRegistry, is_eka2l1};
pub use eka2l1::Eka2l1;
pub use emulator_instance::EmulatorInstance;
pub use emulator_profile::EmulatorProfile;
pub use firmware::Firmware;
pub use registry_entry::RegistryEntry;

#[cfg(test)]
mod tests;
