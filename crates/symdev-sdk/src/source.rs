use crate::{Auth, Index, Result, SdkError, SignedIndex, TrustedKeys};

/// The public bucket's base URL, searched before every configured source: the owner's
/// `symdev-public` R2 bucket on its `r2.dev` address (a custom domain may come later; this
/// address stays enabled so releases that carry it keep working). `None` would mean no
/// built-in source.
const BUILTIN_URL: Option<&str> = Some("https://pub-15670d2771364287b9982e497c29f586.r2.dev/");

/// One place packages come from: an `index.toml` and the archives beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceSpec {
    pub name: String,
    /// Always ends with `/`.
    pub base: String,
    pub auth: Auth,
    /// The keys its `index.toml` must be signed by; `None` reads it unverified, as symdev
    /// 0.1.0 did. The built-in source always has symdev's own keys.
    pub key: Option<TrustedKeys>,
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
            key: None,
        };
        if spec.is_file() && auth == Auth::S3 {
            return Err(SdkError::Other(format!(
                "source `{name}`: `auth = \"s3\"` needs an `https://` URL, not `{url}`"
            )));
        }
        Ok(spec)
    }

    /// The built-in source, named `public`, without authentication, whose index must be
    /// signed by symdev's own keys. The URL is a constant that this module's test checks,
    /// so a malformed one cannot ship.
    pub fn builtin() -> Option<SourceSpec> {
        BUILTIN_URL.and_then(|url| {
            SourceSpec::new("public", url, Auth::None)
                .ok()
                .map(|spec| spec.with_key(TrustedKeys::builtin()))
        })
    }

    /// The same source, accepting only an index that one of `keys` signed.
    pub fn with_key(mut self, keys: TrustedKeys) -> SourceSpec {
        self.key = Some(keys);
        self
    }

    /// The index in `text`, which this source served: with a `key`, only once a signature
    /// by one of its keys is verified (otherwise an error naming [`Self::index_url`]),
    /// and then only the signed bytes are read.
    pub fn parse_index(&self, text: &str) -> Result<Index> {
        match &self.key {
            None => Index::parse(text, &self.name),
            Some(keys) => {
                let signed = SignedIndex::split(text);
                Index::parse(signed.verify(keys, &self.index_url())?, &self.name)
            }
        }
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

    /// Resolves a URL from the source's index against the index's directory. `relative`
    /// must stay under it: not empty or absolute, no scheme (`://`), no `.`, `..` or empty
    /// segment, no `\`, query, fragment, whitespace or control character. So moving a
    /// bucket to another domain changes no byte of its index, and an index cannot point a
    /// download anywhere else.
    pub fn resolve(&self, relative: &str) -> Result<String> {
        if !self.base.ends_with('/') {
            return Err(SdkError::Other(format!(
                "source URL `{}` must end with `/` to resolve `{relative}` against it",
                self.base
            )));
        }
        if let Some(reason) = Self::relative_problem(relative) {
            return Err(SdkError::Other(format!(
                "URL `{relative}` {reason}; an index may only name paths under its own directory"
            )));
        }
        Ok(format!("{}{relative}", self.base))
    }

    /// Why `relative` may not be resolved against an index directory, if it may not.
    pub(crate) fn relative_problem(relative: &str) -> Option<&'static str> {
        if relative.is_empty() {
            Some("is empty")
        } else if relative.starts_with('/') {
            Some("is absolute")
        } else if relative.contains("://") {
            Some("has a scheme")
        } else if relative.contains(['\\', '?', '#']) {
            Some("has a `\\`, `?` or `#`")
        } else if relative
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        {
            Some("has whitespace or a control character")
        } else if relative
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        {
            Some("has an empty, `.` or `..` segment")
        } else {
            None
        }
    }

    /// Whether the source is a local directory (`file://`).
    pub fn is_file(&self) -> bool {
        self.base.starts_with("file://")
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod signature_tests;
