/// Everything the toolchain manager can fail at; each message names what failed and,
/// where one is known, the fix.
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("invalid package id `{id}`: {reason}")]
    InvalidId { id: String, reason: &'static str },
    #[error(
        "source `{source_name}`: index schema {found} is newer than this symdev understands (1); \
         update symdev"
    )]
    UnknownSchema { source_name: String, found: u32 },
    #[error("source `{source_name}`: bad index: {detail}")]
    BadIndex { source_name: String, detail: String },
    #[error("bad sources file {path}: {detail}")]
    BadSources { path: String, detail: String },
    #[error("{url}: {detail}")]
    Fetch { url: String, detail: String },
    #[error(
        "{url}: access denied (HTTP 403) by source `{source_name}`; check the key's bucket \
         permissions"
    )]
    Forbidden { url: String, source_name: String },
    #[error(
        "{id}: downloaded {url} has sha256 {actual} (size {actual_size}), expected {expected} \
         (size {expected_size}); the file was deleted"
    )]
    HashMismatch {
        id: String,
        url: String,
        expected: String,
        actual: String,
        expected_size: u64,
        actual_size: u64,
    },
    #[error("archive {url}: entry `{entry}` {reason}")]
    UnsafeEntry {
        url: String,
        entry: String,
        reason: &'static str,
    },
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SdkError>;

impl SdkError {
    /// A download of `url` that brought more than `limit` bytes, the index's `size`.
    pub(crate) fn longer_than(url: &str, limit: u64) -> SdkError {
        SdkError::Fetch {
            url: url.to_string(),
            detail: format!(
                "stopped after more than the {limit} bytes the index lists; the source's index \
                 or its file is wrong"
            ),
        }
    }
}

impl From<SdkError> for symdev_core::Error {
    fn from(e: SdkError) -> Self {
        symdev_core::Error::Other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::SdkError;

    #[test]
    fn converts_into_the_core_error_with_the_same_message() {
        let e = SdkError::InvalidId {
            id: "gcce".into(),
            reason: "needs a kind and a version",
        };
        let core: symdev_core::Error = e.into();
        assert_eq!(
            core.to_string(),
            "invalid package id `gcce`: needs a kind and a version"
        );
    }
}
