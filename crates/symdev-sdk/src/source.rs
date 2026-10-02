use crate::{Result, SdkError};

/// How requests to a source are authenticated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    #[default]
    None,
    S3,
}

/// One place packages come from: an `index.toml` and the archives beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpec {
    pub name: String,
    /// Always ends with `/`.
    pub base: String,
    pub auth: Auth,
}

impl SourceSpec {
    /// Checks `name` (`[a-z0-9-]+`) and `url`, and adds the trailing `/` to it. A URL is
    /// `https://<host>…`, `file:///<absolute path>`, or plain `http://` to `localhost`,
    /// `127.0.0.1` or `[::1]` only (local mirrors and tests). `Auth::S3` needs HTTP.
    pub fn new(name: &str, url: &str, auth: Auth) -> Result<SourceSpec> {
        let valid_name = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
        if name.is_empty() || !name.chars().all(valid_name) {
            return Err(SdkError::Other(format!(
                "source name `{name}` must match [a-z0-9-]+"
            )));
        }
        if let Some(reason) = Self::url_problem(url) {
            return Err(SdkError::Other(format!(
                "source `{name}`: URL `{url}` {reason}"
            )));
        }
        let base = if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{url}/")
        };
        let spec = SourceSpec {
            name: name.to_string(),
            base,
            auth,
        };
        if spec.is_file() && auth == Auth::S3 {
            return Err(SdkError::Other(format!(
                "source `{name}`: `auth = \"s3\"` needs an `https://` URL, not `{url}`"
            )));
        }
        Ok(spec)
    }

    fn url_problem(url: &str) -> Option<&'static str> {
        if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Some("has whitespace or a control character");
        }
        if let Some(path) = url.strip_prefix("file://") {
            return (!path.starts_with('/'))
                .then_some("needs an absolute path after `file://` (`file:///…`)");
        }
        if let Some(rest) = url.strip_prefix("https://") {
            return (rest.is_empty() || rest.starts_with('/')).then_some("has no host");
        }
        if let Some(rest) = url.strip_prefix("http://") {
            let host_port = rest.split('/').next().unwrap_or_default();
            let local = ["localhost", "127.0.0.1", "[::1]"].iter().any(|host| {
                host_port.strip_prefix(host).is_some_and(|port| {
                    port.is_empty()
                        || port
                            .strip_prefix(':')
                            .is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
                })
            });
            return (!local)
                .then_some("uses plain `http://`, which is only for localhost; use `https://`");
        }
        Some("must start with `https://`, `file://`, or `http://localhost`")
    }

    /// The source's `index.toml`.
    pub fn index_url(&self) -> String {
        format!("{}index.toml", self.base)
    }

    /// Whether the source is a local directory (`file://`).
    pub fn is_file(&self) -> bool {
        self.base.starts_with("file://")
    }
}

#[cfg(test)]
mod tests {
    use super::{Auth, SourceSpec};

    #[test]
    fn adds_the_trailing_slash_and_names_the_index() {
        let s = SourceSpec::new("public", "https://pub-1.r2.dev", Auth::None).unwrap();
        assert_eq!(s.base, "https://pub-1.r2.dev/");
        assert_eq!(s.index_url(), "https://pub-1.r2.dev/index.toml");
        assert!(!s.is_file());
        let f = SourceSpec::new("m", "file:///srv/m/", Auth::None).unwrap();
        assert_eq!(f.index_url(), "file:///srv/m/index.toml");
        assert!(f.is_file());
    }

    #[test]
    fn an_invalid_name_or_url_names_itself() {
        let e = SourceSpec::new("Bad", "file:///x", Auth::None).unwrap_err();
        assert!(e.to_string().contains("`Bad`"), "{e}");
        let e = SourceSpec::new("m", "ftp://x/", Auth::None).unwrap_err();
        assert!(e.to_string().contains("ftp://x/"), "{e}");
    }
}
