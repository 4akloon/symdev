use crate::{Auth, SourceSpec};

/// The public bucket's base URL, searched before every configured source: the owner's
/// `symdev-public` R2 bucket on its `r2.dev` address (a custom domain may come later; this
/// address stays enabled so releases that carry it keep working). `None` would mean no
/// built-in source.
pub const BUILTIN_SOURCE: Option<&str> =
    Some("https://pub-15670d2771364287b9982e497c29f586.r2.dev/");

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
        assert!(BUILTIN_SOURCE.is_some());
        let source = builtin_source().expect("the built-in URL parses as a source");
        assert_eq!(source.name, "public");
        assert_eq!(source.auth, Auth::None);
        assert_eq!(
            source.index_url(),
            format!("{}index.toml", BUILTIN_SOURCE.unwrap_or(""))
        );
    }
}
