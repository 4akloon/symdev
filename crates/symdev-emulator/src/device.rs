//! Devices `cargo run` and `cargo test` run on (design spec §5): emulator profiles.
mod emulator_profile;

pub use emulator_profile::EmulatorProfile;

#[cfg(test)]
mod tests;
