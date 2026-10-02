use std::fs;
use std::io;
use std::path::Path;

use crate::{PackageId, Result, SdkError};

/// What was installed in a package directory and from where: `.symdev-package.toml`,
/// written **last**, so a package directory without one is unfinished.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Receipt {
    pub id: PackageId,
    pub sha256: String,
    /// The name of the source it came from.
    pub source: String,
    /// The archive's full URL.
    pub url: String,
}

impl Receipt {
    pub(crate) const FILE: &str = ".symdev-package.toml";

    /// The receipt in `dir`, or `None` if there is none (not installed, or unfinished).
    pub fn read(dir: &Path) -> Result<Option<Receipt>> {
        let path = dir.join(Self::FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                let path = path.display().to_string();
                return Err(SdkError::Io { path, source });
            }
        };
        toml::from_str(&text).map(Some).map_err(|e| {
            SdkError::Other(format!(
                "{}: damaged receipt ({}); remove {} and run `symdev sdk install` again",
                path.display(),
                e.message(),
                dir.display()
            ))
        })
    }

    /// Writes the receipt into `dir` atomically (a temporary file renamed over it).
    pub fn write(&self, dir: &Path) -> Result<()> {
        let text = toml::to_string(self).map_err(|e| {
            SdkError::Other(format!("cannot write the receipt of {}: {e}", self.id))
        })?;
        let path = dir.join(Self::FILE);
        let partial = dir.join(format!("{}.partial", Self::FILE));
        let io_at = |p: &Path| {
            let p = p.display().to_string();
            move |source| SdkError::Io { path: p, source }
        };
        fs::write(&partial, text).map_err(io_at(&partial))?;
        fs::rename(&partial, &path).map_err(io_at(&path))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Receipt;
    use crate::PackageId;

    fn receipt() -> Receipt {
        Receipt {
            id: PackageId::parse("gcce;12.1.0").unwrap(),
            sha256: "ab".repeat(32),
            source: "public".into(),
            url: "https://pub-1.r2.dev/gcce/12.1.0/x.tar.gz".into(),
        }
    }

    #[test]
    fn a_directory_without_a_receipt_has_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(Receipt::read(tmp.path()).unwrap(), None);
        assert_eq!(Receipt::read(&tmp.path().join("missing")).unwrap(), None);
    }

    #[test]
    fn writes_the_four_keys_and_reads_them_back() {
        let tmp = tempfile::tempdir().unwrap();
        receipt().write(tmp.path()).unwrap();
        let text = fs::read_to_string(tmp.path().join(".symdev-package.toml")).unwrap();
        assert!(text.contains("id = \"gcce;12.1.0\""), "{text}");
        assert!(text.contains("source = \"public\""), "{text}");
        assert_eq!(Receipt::read(tmp.path()).unwrap(), Some(receipt()));
        let names: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
        assert_eq!(names.len(), 1, "no temporary file is left behind");
    }

    #[test]
    fn a_damaged_receipt_names_the_file_and_the_fix() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join(".symdev-package.toml"), "id = 3").unwrap();
        let e = Receipt::read(tmp.path()).unwrap_err().to_string();
        assert!(e.contains(".symdev-package.toml"), "{e}");
        assert!(e.contains("symdev sdk install"), "{e}");
    }
}
