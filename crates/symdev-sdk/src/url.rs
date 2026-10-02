use crate::{Result, SdkError};

/// Resolves a URL from an index against the index's directory. `base` ends with `/`;
/// `relative` must stay under it: not empty or absolute, no scheme (`://`), no `.`, `..`
/// or empty segment, no `\`, query, fragment, whitespace or control character. So moving
/// a bucket to another domain changes no byte of its index, and an index cannot point a
/// download anywhere else.
pub fn resolve_url(base: &str, relative: &str) -> Result<String> {
    if !base.ends_with('/') {
        return Err(SdkError::Other(format!(
            "source URL `{base}` must end with `/` to resolve `{relative}` against it"
        )));
    }
    if let Some(reason) = relative_url_problem(relative) {
        return Err(SdkError::Other(format!(
            "URL `{relative}` {reason}; an index may only name paths under its own directory"
        )));
    }
    Ok(format!("{base}{relative}"))
}

/// Why `relative` may not be resolved against an index directory, if it may not.
pub(crate) fn relative_url_problem(relative: &str) -> Option<&'static str> {
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

#[cfg(test)]
mod tests {
    use super::resolve_url;

    #[test]
    fn joins_a_relative_path_to_the_index_directory() {
        assert_eq!(
            resolve_url("https://x/b/", "gcce/1/a.tar.gz").unwrap(),
            "https://x/b/gcce/1/a.tar.gz"
        );
        assert_eq!(
            resolve_url("file:///srv/mirror/", "a..b.tar.gz").unwrap(),
            "file:///srv/mirror/a..b.tar.gz"
        );
    }

    #[test]
    fn refuses_paths_that_leave_the_index_directory() {
        for relative in [
            "../a",
            "/a",
            "https://evil/a",
            "a/../b",
            "a/./b",
            "a//b",
            "",
            "a\\b",
        ] {
            let e = resolve_url("https://x/b/", relative)
                .unwrap_err()
                .to_string();
            assert!(e.contains(&format!("`{relative}`")), "{relative}: {e}");
        }
    }

    #[test]
    fn refuses_query_fragment_and_whitespace() {
        for relative in ["a?x=1", "a#f", "a b", "a\nb"] {
            assert!(
                resolve_url("https://x/b/", relative).is_err(),
                "{relative:?}"
            );
        }
    }

    #[test]
    fn refuses_a_base_without_a_trailing_slash() {
        let e = resolve_url("https://x/b", "a").unwrap_err().to_string();
        assert!(e.contains("https://x/b"), "{e}");
    }
}
