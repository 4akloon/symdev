use crate::{Error, Result};

/// Platform security capabilities as the bit set E32 (`iCaps`) and SIS (type 41) store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    bits: u64,
}

impl Capabilities {
    /// Bit per capability name, recorded from elf2e32_next `iCaps` one name at a time
    /// (experiment 50); SIS type 41 uses the same bits (Wine makesis, experiment 50).
    const BITS: &[(&str, u32)] = &[
        ("TCB", 0),
        ("CommDD", 1),
        ("PowerMgmt", 2),
        ("MultimediaDD", 3),
        ("ReadDeviceData", 4),
        ("WriteDeviceData", 5),
        ("DRM", 6),
        ("TrustedUI", 7),
        ("ProtServ", 8),
        ("DiskAdmin", 9),
        ("NetworkControl", 10),
        ("AllFiles", 11),
        ("SwEvent", 12),
        ("NetworkServices", 13),
        ("LocalServices", 14),
        ("ReadUserData", 15),
        ("WriteUserData", 16),
        ("Location", 17),
        ("SurroundingsDD", 18),
        ("UserEnvironment", 19),
    ];

    pub fn from_names<S: AsRef<str>>(names: &[S]) -> Result<Self> {
        let mut bits = 0u64;
        for name in names {
            let name = name.as_ref();
            let Some((_, bit)) = Self::BITS.iter().find(|(n, _)| *n == name) else {
                return Err(Error::Other(format!(
                    "capability bit not yet derived: {name}"
                )));
            };
            bits |= 1 << bit;
        }
        Ok(Self { bits })
    }

    pub fn bits(&self) -> u64 {
        self.bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_six_user_grantable_match_experiment_6() {
        let caps = Capabilities::from_names(&[
            "LocalServices",
            "NetworkServices",
            "ReadUserData",
            "WriteUserData",
            "UserEnvironment",
            "Location",
        ])
        .unwrap();
        assert_eq!(caps.bits(), 0x000b_e000);
    }

    #[test]
    fn experiment_50_privileged_mix_matches_elf2e32_and_makesis() {
        let caps =
            Capabilities::from_names(&["TCB", "AllFiles", "ReadDeviceData", "UserEnvironment"])
                .unwrap();
        assert_eq!(caps.bits(), 0x0008_0811);
    }

    #[test]
    fn unknown_capability_is_an_error() {
        let err = Capabilities::from_names(&["NotACapability"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("NotACapability"), "{err}");
    }
}
