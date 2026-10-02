use crate::{Result, SdkError};

/// The parts of an `http(s)://` URL that SigV4 signs: the `Host` header value as the HTTP
/// client sends it, and the canonical path and query string.
///
/// S3 rules: the path is not normalised. Whatever the URL holds is percent-decoded first and
/// then encoded once with SigV4's unreserved set, so `a;b` and `a%3Bb` both sign as `a%3Bb`
/// (the key the server decodes) and nothing is encoded twice.
pub(crate) struct RequestTarget {
    pub(crate) host: String,
    pub(crate) canonical_uri: String,
    pub(crate) canonical_query: String,
}

impl RequestTarget {
    pub(crate) fn parse(url: &str) -> Result<RequestTarget> {
        let refuse = |detail: &str| SdkError::Fetch {
            url: url.to_string(),
            detail: format!("cannot sign the request: {detail}"),
        };
        let (scheme, rest) = url
            .split_once("://")
            .ok_or_else(|| refuse("the URL has no scheme; use an https:// URL"))?;
        let default_port = match scheme.to_ascii_lowercase().as_str() {
            "https" => "443",
            "http" => "80",
            _ => return Err(refuse("only http:// and https:// URLs can be signed")),
        };
        let rest = rest.split('#').next().unwrap_or_default();
        let (authority, path_and_query) =
            rest.split_at(rest.find(['/', '?']).unwrap_or(rest.len()));
        if authority.is_empty() {
            return Err(refuse("the URL has no host"));
        }
        if authority.contains('@') {
            return Err(refuse(
                "user info in the URL is not supported; pass keys separately",
            ));
        }
        // The HTTP client sends the port only when it is not the scheme's default.
        let host = match authority.rsplit_once(':') {
            Some((host, port)) if port == default_port && !host.is_empty() => host,
            _ => authority,
        };
        let (path, query) = path_and_query
            .split_once('?')
            .unwrap_or((path_and_query, ""));
        let bad_escape = || refuse("it holds a `%` that is not followed by two hex digits");
        let canonical_uri = if path.is_empty() {
            "/".to_string()
        } else {
            encode(&decode(path).ok_or_else(bad_escape)?, true)
        };
        let mut pairs = Vec::new();
        for pair in query.split('&').filter(|pair| !pair.is_empty()) {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let name = encode(&decode(name).ok_or_else(bad_escape)?, false);
            let value = encode(&decode(value).ok_or_else(bad_escape)?, false);
            pairs.push((name, value));
        }
        pairs.sort();
        let canonical_query = pairs
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("&");
        Ok(RequestTarget {
            host: host.to_string(),
            canonical_uri,
            canonical_query,
        })
    }
}

/// `%XX` escapes turned into bytes; `None` for a `%` without two hex digits after it.
fn decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let digits = bytes.get(i + 1..i + 3)?;
            if !digits.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let hex = std::str::from_utf8(digits).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}

/// SigV4's `UriEncode`: unreserved bytes stay, every other byte becomes `%XX` (upper-case);
/// `/` stays only in a path.
fn encode(bytes: &[u8], keep_slash: bool) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) || (keep_slash && b == b'/') {
            out.push(char::from(b));
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests;
