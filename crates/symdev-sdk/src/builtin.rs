use crate::{Auth, SourceSpec};

/// The public bucket's base URL, searched before every configured source. Set in task R1
/// once the bucket exists; `None` means this symdev has no built-in source.
pub const BUILTIN_SOURCE: Option<&str> = None;

/// The built-in source, named `public`, without authentication. The URL is a constant
/// that this module's test checks, so a malformed one cannot ship.
pub fn builtin_source() -> Option<SourceSpec> {
    BUILTIN_SOURCE.and_then(|url| SourceSpec::new("public", url, Auth::None).ok())
}

#[cfg(test)]
mod tests {
    use super::{BUILTIN_SOURCE, builtin_source};
    use crate::Auth;

    #[test]
    fn the_built_in_url_is_a_valid_public_source() {
        assert_eq!(builtin_source().is_some(), BUILTIN_SOURCE.is_some());
        if let Some(source) = builtin_source() {
            assert_eq!(source.name, "public");
            assert_eq!(source.auth, Auth::None);
        }
    }
}
