/// An S3 key pair for one source. The CLI reads it from the environment; this crate
/// never does.
#[derive(Clone, PartialEq, Eq)]
pub struct S3Keys {
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl S3Keys {
    /// The two variables that hold a source's keys:
    /// `SYMDEV_SOURCE_<NAME>_ACCESS_KEY_ID` and `SYMDEV_SOURCE_<NAME>_SECRET_ACCESS_KEY`,
    /// with the name upper-cased and `-` turned into `_`.
    pub fn variable_names(source_name: &str) -> (String, String) {
        let name = source_name.to_ascii_uppercase().replace('-', "_");
        (
            format!("SYMDEV_SOURCE_{name}_ACCESS_KEY_ID"),
            format!("SYMDEV_SOURCE_{name}_SECRET_ACCESS_KEY"),
        )
    }
}

/// Never prints the secret, so a key pair can sit in a `Debug`-printed struct.
impl std::fmt::Debug for S3Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S3Keys")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"***")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::S3Keys;

    #[test]
    fn names_the_two_variables_of_a_source() {
        assert_eq!(
            S3Keys::variable_names("private"),
            (
                "SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID".to_string(),
                "SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY".to_string()
            )
        );
        assert_eq!(
            S3Keys::variable_names("my-mirror").0,
            "SYMDEV_SOURCE_MY_MIRROR_ACCESS_KEY_ID"
        );
    }

    #[test]
    fn debug_hides_the_secret() {
        let keys = S3Keys {
            access_key_id: "AKID".into(),
            secret_access_key: "very-secret".into(),
        };
        let shown = format!("{keys:?}");
        assert!(shown.contains("AKID"));
        assert!(!shown.contains("very-secret"));
    }
}
