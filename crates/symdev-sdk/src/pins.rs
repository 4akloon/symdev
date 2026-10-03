use symdev_manifest::Device;

use crate::PackageId;

/// The package versions this symdev release builds with, and the only package ids
/// written in the code. CI keys its `SYMDEV_HOME` cache on the hash of this file.
pub struct Pins;

impl Pins {
    /// The compiler, pinned by the symdev release (as AGP pins its default NDK).
    pub fn gcce() -> PackageId {
        PackageId::pinned("gcce;12.1.0")
    }

    /// The platform SDK that `[target] device` builds against.
    pub fn platform_sdk(device: Device) -> PackageId {
        match device {
            Device::NokiaE52 => PackageId::pinned("sdk;s60-3rd-fp2;1.1"),
        }
    }

    /// The Rust SDK (`symbian-rs`) published with this symdev release: the two come from
    /// one tag, so the version is the workspace's (spec §12).
    pub fn rust_sdk() -> PackageId {
        PackageId::pinned(concat!("rust-sdk;", env!("CARGO_PKG_VERSION")))
    }

    /// The EKA2L1 `cargo run` starts when `SYMDEV_EKA2L1` is not set: the fork CI's build of
    /// the integration branch this release was tested with (emulator packages spec §3).
    pub fn emulator() -> PackageId {
        PackageId::pinned("emulator;2026.10.03")
    }

    /// The firmware an emulator profile of `device` is made from when `SYMDEV_EKA2L1_DATA`
    /// is not set (emulator packages spec §4). Only a private source has it.
    pub fn firmware(device: Device) -> PackageId {
        match device {
            Device::NokiaE52 => PackageId::pinned("firmware;rm-469;1"),
        }
    }
}

#[cfg(test)]
mod tests {
    use symdev_manifest::Device;

    use super::Pins;
    use crate::PackageId;

    #[test]
    fn pins_gcce_12_1_0() {
        assert_eq!(Pins::gcce(), PackageId::parse("gcce;12.1.0").unwrap());
    }

    #[test]
    fn the_e52_builds_with_the_s60_3rd_fp2_sdk() {
        assert_eq!(
            Pins::platform_sdk(Device::NokiaE52),
            PackageId::parse("sdk;s60-3rd-fp2;1.1").unwrap()
        );
    }

    #[test]
    fn the_e52_runs_on_the_rm_469_firmware() {
        assert_eq!(
            Pins::firmware(Device::NokiaE52),
            PackageId::parse("firmware;rm-469;1").unwrap()
        );
    }

    #[test]
    fn the_emulator_is_pinned_to_a_dated_build() {
        let id = Pins::emulator();
        let segments: Vec<&str> = id.segments().collect();
        assert_eq!(segments.len(), 2, "{id}");
        assert_eq!(segments[0], "emulator");
        let date: Vec<&str> = segments[1].split('.').collect();
        let widths: Vec<usize> = date.iter().map(|part| part.len()).collect();
        assert_eq!(widths, [4, 2, 2], "{id}");
        assert!(
            date.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())),
            "{id}"
        );
    }

    #[test]
    fn every_device_has_a_firmware() {
        for device in Device::ALL {
            assert_eq!(Pins::firmware(device).kind(), "firmware");
        }
    }

    /// `version` under `[workspace.package]` in the workspace's `Cargo.toml`.
    fn workspace_version() -> String {
        let manifest: toml::Table = include_str!("../../../Cargo.toml").parse().unwrap();
        manifest["workspace"]["package"]["version"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn the_rust_sdk_is_the_one_of_this_symdev_release() {
        let expected = format!("rust-sdk;{}", workspace_version());
        assert_eq!(Pins::rust_sdk(), PackageId::parse(&expected).unwrap());
    }

    #[test]
    fn every_pin_is_a_valid_id() {
        let pins = [
            Pins::gcce(),
            Pins::platform_sdk(Device::NokiaE52),
            Pins::rust_sdk(),
            Pins::emulator(),
            Pins::firmware(Device::NokiaE52),
        ];
        for pinned in pins {
            assert_eq!(PackageId::parse(pinned.as_str()).unwrap(), pinned);
        }
    }
}
