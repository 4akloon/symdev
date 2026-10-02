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
        ];
        for pinned in pins {
            assert_eq!(PackageId::parse(pinned.as_str()).unwrap(), pinned);
        }
    }
}
