mod canonical_request;
mod request_target;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use self::canonical_request::CanonicalRequest;
use self::request_target::RequestTarget;
use crate::{AmzDate, Result, S3Keys, SdkError};

/// The headers this signer sets itself; an `extra` header may not repeat them.
const OWN_HEADERS: [&str; 4] = [
    "host",
    "x-amz-content-sha256",
    "x-amz-date",
    "authorization",
];

/// AWS Signature Version 4 in the `Authorization` header, for S3-compatible GET and PUT
/// requests (Cloudflare R2 takes region `auto`).
pub struct SigV4 {
    keys: S3Keys,
    region: String,
    service: String,
}

impl SigV4 {
    /// A signer for the `s3` service in `region`.
    pub fn s3(keys: S3Keys, region: &str) -> SigV4 {
        SigV4 {
            keys,
            region: region.to_string(),
            service: "s3".to_string(),
        }
    }

    /// The headers to add to the request: `x-amz-date`, `x-amz-content-sha256` and
    /// `authorization`, in that order. Signs `host`, `x-amz-content-sha256`, `x-amz-date`
    /// and every `extra` header (names lower-cased and sorted).
    pub fn sign(
        &self,
        method: &str,
        url: &str,
        extra: &[(&str, &str)],
        payload_sha256: &str,
        now: &AmzDate,
    ) -> Result<Vec<(String, String)>> {
        let headers = self.canonical_headers(url, extra, payload_sha256, now)?;
        let canonical = CanonicalRequest::new(method, url, &headers, payload_sha256)?;
        Ok(vec![
            ("x-amz-date".to_string(), now.as_str().to_string()),
            (
                "x-amz-content-sha256".to_string(),
                payload_sha256.to_string(),
            ),
            (
                "authorization".to_string(),
                self.authorization(now, &canonical)?,
            ),
        ])
    }

    /// Every header `sign` signs, `host` first, as the HTTP client will send them.
    fn canonical_headers(
        &self,
        url: &str,
        extra: &[(&str, &str)],
        payload_sha256: &str,
        now: &AmzDate,
    ) -> Result<Vec<(String, String)>> {
        let mut headers = vec![
            ("host".to_string(), RequestTarget::parse(url)?.host),
            (
                "x-amz-content-sha256".to_string(),
                payload_sha256.to_string(),
            ),
            ("x-amz-date".to_string(), now.as_str().to_string()),
        ];
        for (name, value) in extra {
            let name = name.to_ascii_lowercase();
            if OWN_HEADERS.contains(&name.as_str()) {
                return Err(SdkError::Other(format!(
                    "header `{name}` is set by the SigV4 signer; do not pass it as an extra header"
                )));
            }
            headers.push((name, value.to_string()));
        }
        Ok(headers)
    }

    pub(crate) fn string_to_sign(&self, now: &AmzDate, canonical: &CanonicalRequest) -> String {
        format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            now.as_str(),
            self.scope(now),
            hex(&Sha256::digest(canonical.text().as_bytes()))
        )
    }

    /// The hex HMAC of `string_to_sign` under the key derived for `now`'s day.
    pub(crate) fn signature(&self, now: &AmzDate, string_to_sign: &str) -> Result<String> {
        let secret = format!("AWS4{}", self.keys.secret_access_key);
        let mut key = hmac(secret.as_bytes(), now.day().as_bytes())?;
        for part in [self.region.as_str(), self.service.as_str(), "aws4_request"] {
            key = hmac(&key, part.as_bytes())?;
        }
        Ok(hex(&hmac(&key, string_to_sign.as_bytes())?))
    }

    /// The `Authorization` header value for `canonical`.
    pub(crate) fn authorization(
        &self,
        now: &AmzDate,
        canonical: &CanonicalRequest,
    ) -> Result<String> {
        let signature = self.signature(now, &self.string_to_sign(now, canonical))?;
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={signature}",
            self.keys.access_key_id,
            self.scope(now),
            canonical.signed_headers()
        ))
    }

    fn scope(&self, now: &AmzDate) -> String {
        format!(
            "{}/{}/{}/aws4_request",
            now.day(),
            self.region,
            self.service
        )
    }
}

fn hmac(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).map_err(|e| {
        SdkError::Other(format!("HMAC-SHA256 refused a {}-byte key: {e}", key.len()))
    })?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests;
