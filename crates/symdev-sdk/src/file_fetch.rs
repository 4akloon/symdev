use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use crate::{Fetch, Result, SdkError};

/// Reads a `file://` source: a local directory holding `index.toml` and archives (tests,
/// local mirrors). The path after `file://` is used as is, without percent-decoding.
pub struct FileFetch;

impl FileFetch {
    fn path(url: &str) -> Result<&Path> {
        let path = url.strip_prefix("file://").filter(|p| p.starts_with('/'));
        path.map(Path::new).ok_or_else(|| SdkError::Fetch {
            url: url.to_string(),
            detail: "not a `file:///…` URL, the only kind a local source reads".into(),
        })
    }
}

impl Fetch for FileFetch {
    fn text(&self, url: &str) -> Result<String> {
        fs::read_to_string(Self::path(url)?).map_err(|e| SdkError::Fetch {
            url: url.to_string(),
            detail: e.to_string(),
        })
    }

    fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<u64> {
        let from = File::open(Self::path(url)?).map_err(|e| SdkError::Fetch {
            url: url.to_string(),
            detail: e.to_string(),
        })?;
        let io_at_dest = |source| SdkError::Io {
            path: dest.display().to_string(),
            source,
        };
        let mut to = File::create(dest).map_err(io_at_dest)?;
        let copied = io::copy(&mut from.take(limit.saturating_add(1)), &mut to);
        match copied.map_err(io_at_dest)? {
            copied if copied > limit => Err(SdkError::longer_than(url, limit)),
            copied => Ok(copied),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::FileFetch;
    use crate::{Fetch, SdkError};

    #[test]
    fn reads_text_and_copies_files() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("index.toml"), "schema = 1\n").unwrap();
        fs::write(tmp.path().join("a.tar.gz"), b"\x1f\x8bdata").unwrap();
        let base = format!("file://{}/", tmp.path().display());
        assert_eq!(
            FileFetch.text(&format!("{base}index.toml")).unwrap(),
            "schema = 1\n"
        );
        let dest = tmp.path().join("out");
        fs::write(&dest, b"longer old content").unwrap();
        assert_eq!(
            FileFetch
                .download(&format!("{base}a.tar.gz"), &dest, 6)
                .unwrap(),
            6
        );
        assert_eq!(fs::read(&dest).unwrap(), b"\x1f\x8bdata");
    }

    #[test]
    fn a_file_longer_than_its_limit_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("a.tar.gz"), b"0123456789").unwrap();
        let url = format!("file://{}/a.tar.gz", tmp.path().display());
        match FileFetch.download(&url, &tmp.path().join("out"), 4) {
            Err(SdkError::Fetch { url: u, detail }) => {
                assert_eq!(u, url);
                assert!(detail.contains("more than the 4 bytes"), "{detail}");
            }
            other => panic!("expected Fetch, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_file_is_a_fetch_error_naming_the_url() {
        let url = "file:///nonexistent/symdev/index.toml";
        match FileFetch.text(url) {
            Err(SdkError::Fetch { url: u, .. }) => assert_eq!(u, url),
            other => panic!("expected Fetch, got {other:?}"),
        }
    }

    #[test]
    fn refuses_a_url_that_is_not_file() {
        let e = FileFetch
            .text("https://x/index.toml")
            .unwrap_err()
            .to_string();
        assert!(e.contains("https://x/index.toml"), "{e}");
        assert!(e.contains("file://"), "{e}");
    }
}
