use crate::Host;

/// One downloadable archive of a package, for one host.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveEntry {
    pub host: Host,
    /// Relative to the index's directory.
    pub url: String,
    pub sha256: String,
    pub size: u64,
}
