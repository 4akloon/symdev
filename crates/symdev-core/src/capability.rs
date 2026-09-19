use crate::{Error, Result};

/// Platform security capabilities as the bit set E32 (`iCaps`) and SIS (type 41) store.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Capabilities {
    bits: u64,
}

impl Capabilities {
    /// Bits recorded for the six user-grantable names (experiment 6: hello
    /// `iCaps` / SIS type-41 `0x000be000`). Other names are not derived yet.
    const BITS: &[(&str, u32)] = &[
        ("NetworkServices", 13),
        ("LocalServices", 14),
        ("ReadUserData", 15),
        ("WriteUserData", 16),
        ("Location", 17),
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
    fn unknown_capability_is_an_error() {
        let err = Capabilities::from_names(&["AllFiles"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("AllFiles"), "{err}");
    }
}
