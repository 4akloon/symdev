use std::fmt::Display;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;
use std::time::Duration;

use ureq::http::Response;
use ureq::{Agent, Body};

use crate::{AmzDate, Fetch, Result, SdkError, SigV4};

/// SHA-256 of an empty body, the payload hash of every GET.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// An `index.toml` larger than this is refused rather than read into memory.
const MAX_TEXT_BYTES: u64 = 10 * 1024 * 1024;

/// The HTTP(S) adapter for one source: plain requests, or S3 requests signed with SigV4
/// when the source has keys.
///
/// The proxy comes from `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` and `NO_PROXY` (ureq's
/// environment support). Connecting may take 30 s and a whole request an hour: a 60 MB
/// archive then still arrives over a 20 KB/s link, while a stalled one does not hang a
/// build for ever (ureq has no idle timeout to use instead). A non-2xx
/// answer is `SdkError::Forbidden` for 403 and otherwise `SdkError::Fetch` whose `detail` is
/// exactly `HTTP <code>`, so a caller can tell a missing object (`HTTP 404`) apart.
pub struct HttpFetch {
    agent: Agent,
    signer: Option<SigV4>,
    source_name: String,
}

impl HttpFetch {
    /// `source_name` appears in errors; `signer` signs every request when present.
    pub fn new(source_name: &str, signer: Option<SigV4>) -> HttpFetch {
        let config = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_global(Some(Duration::from_secs(60 * 60)))
            .build();
        HttpFetch {
            agent: Agent::new_with_config(config),
            signer,
            source_name: source_name.to_string(),
        }
    }

    /// Uploads `file` to `url` with a PUT, streamed with its length, signed when the source
    /// has keys. `sha256` is the file's hex SHA-256: it is signed as the payload hash, so the
    /// bucket refuses the upload if the bytes it receives differ. `content_type` and
    /// `cache_control` are sent and signed.
    pub fn put_file(
        &self,
        url: &str,
        file: &Path,
        sha256: &str,
        content_type: &str,
        cache_control: &str,
    ) -> Result<()> {
        let body = File::open(file).map_err(|source| SdkError::Io {
            path: file.display().to_string(),
            source,
        })?;
        let extra = [
            ("content-type", content_type),
            ("cache-control", cache_control),
        ];
        let mut request = self.agent.put(url);
        for (name, value) in extra {
            request = request.header(name, value);
        }
        for (name, value) in self.signed_headers("PUT", url, &extra, sha256)? {
            request = request.header(name, value);
        }
        let response = request.send(body).map_err(|e| failed(url, e))?;
        self.success(url, response).map(drop)
    }

    fn get(&self, url: &str) -> Result<Response<Body>> {
        let mut request = self.agent.get(url);
        for (name, value) in self.signed_headers("GET", url, &[], EMPTY_SHA256)? {
            request = request.header(name, value);
        }
        let response = request.call().map_err(|e| failed(url, e))?;
        self.success(url, response)
    }

    /// The SigV4 headers for the request, or none for an unsigned source.
    fn signed_headers(
        &self,
        method: &str,
        url: &str,
        extra: &[(&str, &str)],
        payload_sha256: &str,
    ) -> Result<Vec<(String, String)>> {
        match &self.signer {
            Some(signer) => signer.sign(method, url, extra, payload_sha256, &AmzDate::now()),
            None => Ok(Vec::new()),
        }
    }

    fn success(&self, url: &str, response: Response<Body>) -> Result<Response<Body>> {
        match response.status().as_u16() {
            200..=299 => Ok(response),
            403 => Err(SdkError::Forbidden {
                url: url.to_string(),
                source_name: self.source_name.clone(),
            }),
            code => Err(failed(url, format!("HTTP {code}"))),
        }
    }
}

impl Fetch for HttpFetch {
    fn text(&self, url: &str) -> Result<String> {
        self.get(url)?
            .body_mut()
            .with_config()
            .limit(MAX_TEXT_BYTES)
            .read_to_string()
            .map_err(|e| failed(url, e))
    }

    fn download(&self, url: &str, dest: &Path) -> Result<u64> {
        let mut response = self.get(url)?;
        let disk = |source| SdkError::Io {
            path: dest.display().to_string(),
            source,
        };
        let mut file = File::create(dest).map_err(disk)?;
        let mut body = response.body_mut().as_reader();
        let mut buffer = vec![0; 64 * 1024];
        let mut written = 0u64;
        loop {
            let read = match body.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => {
                    let detail = format!("download stopped after {written} bytes: {e}");
                    return Err(failed(url, detail));
                }
            };
            file.write_all(&buffer[..read]).map_err(disk)?;
            written += read as u64;
        }
        Ok(written)
    }
}

fn failed(url: &str, detail: impl Display) -> SdkError {
    SdkError::Fetch {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
