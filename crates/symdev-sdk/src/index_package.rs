use crate::{Host, PackageId};

/// One downloadable archive of a package, for one host.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveEntry {
    pub host: Host,
    /// Relative to the index's directory.
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// A `[[package]]` of an index.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IndexPackage {
    pub id: PackageId,
    pub license: String,
    #[serde(
        rename = "source-code",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub source_code: Option<String>,
    #[serde(default)]
    pub depends: Vec<PackageId>,
    #[serde(rename = "archive", default)]
    pub archives: Vec<ArchiveEntry>,
}

impl IndexPackage {
    /// The archive built for `host`, else the host-independent one.
    pub fn archive_for(&self, host: Host) -> Option<&ArchiveEntry> {
        let on = |h: Host| self.archives.iter().find(|a| a.host == h);
        on(host).or_else(|| on(Host::Any))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchiveEntry, IndexPackage};
    use crate::{Host, PackageId};

    fn archive(host: Host, url: &str) -> ArchiveEntry {
        ArchiveEntry {
            host,
            url: url.into(),
            sha256: "0".repeat(64),
            size: 1,
        }
    }

    fn package(archives: Vec<ArchiveEntry>) -> IndexPackage {
        IndexPackage {
            id: PackageId::parse("gcce;12.1.0").unwrap(),
            license: "GPL-3.0-or-later".into(),
            source_code: None,
            depends: vec![],
            archives,
        }
    }

    #[test]
    fn picks_the_archive_of_the_exact_host() {
        let p = package(vec![
            archive(Host::Any, "any"),
            archive(Host::X86_64Linux, "linux"),
        ]);
        assert_eq!(p.archive_for(Host::X86_64Linux).unwrap().url, "linux");
    }

    #[test]
    fn falls_back_to_the_any_archive() {
        let p = package(vec![archive(Host::Any, "any")]);
        assert_eq!(p.archive_for(Host::X86_64Linux).unwrap().url, "any");
    }

    #[test]
    fn has_no_archive_when_only_another_host_is_listed() {
        let p = package(vec![archive(Host::X86_64Linux, "linux")]);
        assert_eq!(p.archive_for(Host::Any), None);
        let none = package(vec![]);
        assert_eq!(none.archive_for(Host::X86_64Linux), None);
    }
}
