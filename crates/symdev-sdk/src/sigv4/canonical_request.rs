use super::request_target::RequestTarget;
use crate::Result;

/// SigV4's canonical form of one request: method, canonical URI and query, the signed
/// headers and the payload hash, one per line.
pub(crate) struct CanonicalRequest {
    text: String,
    signed_headers: String,
}

impl CanonicalRequest {
    /// `headers` are every header to sign, `host` included, in any case and order.
    pub(crate) fn new(
        method: &str,
        url: &str,
        headers: &[(String, String)],
        payload_sha256: &str,
    ) -> Result<CanonicalRequest> {
        let target = RequestTarget::parse(url)?;
        let mut canonical: Vec<(String, String)> = Vec::with_capacity(headers.len());
        for (name, value) in headers {
            let name = name.to_ascii_lowercase();
            let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
            match canonical.iter_mut().find(|(seen, _)| *seen == name) {
                Some((_, values)) => {
                    values.push(',');
                    values.push_str(&value);
                }
                None => canonical.push((name, value)),
            }
        }
        canonical.sort();
        let block: String = canonical
            .iter()
            .map(|(name, value)| format!("{name}:{value}\n"))
            .collect();
        let signed_headers = canonical
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(";");
        let text = format!(
            "{method}\n{}\n{}\n{block}\n{signed_headers}\n{payload_sha256}",
            target.canonical_uri, target.canonical_query
        );
        Ok(CanonicalRequest {
            text,
            signed_headers,
        })
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    /// The lower-cased header names, sorted and joined with `;`.
    pub(crate) fn signed_headers(&self) -> &str {
        &self.signed_headers
    }
}
