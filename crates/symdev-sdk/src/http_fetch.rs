use std::fmt::Display;
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;

use ureq::http::Response;
use ureq::{Agent, Body, Proxy};

use crate::http_timeouts::HttpTimeouts;
use crate::{AmzDate, Fetch, Result, SdkError, SigV4, SourceSpec};

/// SHA-256 of an empty body, the payload hash of every GET.
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// An `index.toml` larger than this is refused rather than read into memory.
const MAX_TEXT_BYTES: u64 = 10 * 1024 * 1024;

/// The HTTP(S) adapter for one source: plain requests, or S3 requests signed with SigV4
/// when the source has keys.
///
/// The proxy comes from `ALL_PROXY` / `HTTPS_PROXY` / `HTTP_PROXY` and `NO_PROXY` (ureq's
/// environment support). Connecting may take 30 s, the server's answer (its response
/// headers) 60 s, and the body a minute plus its size at 16 KiB/s ([`HttpTimeouts`]):
/// the index's `size` for a download, the 10 MiB cap for an index. ureq has no idle
/// timeout, so a server that sends the headers and then stalls is given up on once a
/// 16 KiB/s link would have delivered the whole body (70 minutes for the 67 MB GCCE). A
/// whole request may take an hour, or longer when its body may. A download stops as soon
/// as it passes its limit. An `https` source sends no plain-HTTP request, so no redirect
/// can downgrade it, and a signed request follows no redirect at all. A non-2xx answer (a
/// redirect not followed too) is `SdkError::Forbidden` for 403 and otherwise
/// `SdkError::Fetch` whose `detail` is exactly `HTTP <code>`, so a caller can tell a
/// missing object (`HTTP 404`) apart.
pub struct HttpFetch {
    agent: Agent,
    signer: Option<SigV4>,
    source_name: String,
    timeouts: HttpTimeouts,
}

impl HttpFetch {
    /// The adapter for `source`, whose name appears in errors; `signer` signs every
    /// request when present. The proxy is the environment's.
    pub fn new(source: &SourceSpec, signer: Option<SigV4>) -> HttpFetch {
        let https = source.base.starts_with("https://");
        Self::with(
            &source.name,
            https,
            signer,
            Proxy::try_from_env(),
            HttpTimeouts::STANDARD,
        )
    }

    /// For a plain-`http` test source named `source_name`, without any proxy whatever the
    /// environment holds: the tests' servers listen on 127.0.0.1, and a developer's
    /// `HTTP_PROXY` must not take their requests.
    #[cfg(test)]
    pub(crate) fn direct(source_name: &str, signer: Option<SigV4>) -> HttpFetch {
        Self::with(source_name, false, signer, None, HttpTimeouts::STANDARD)
    }

    /// [`Self::new`] without any proxy, for the tests' servers on 127.0.0.1.
    #[cfg(test)]
    pub(crate) fn direct_for(source: &SourceSpec, signer: Option<SigV4>) -> HttpFetch {
        let https = source.base.starts_with("https://");
        Self::with(&source.name, https, signer, None, HttpTimeouts::STANDARD)
    }

    /// [`Self::direct`] waiting only `wait` for a response, so a test of a silent server
    /// ends in a moment.
    #[cfg(test)]
    pub(crate) fn impatient(
        source_name: &str,
        signer: Option<SigV4>,
        wait: std::time::Duration,
    ) -> HttpFetch {
        let timeouts = HttpTimeouts {
            response: wait,
            ..HttpTimeouts::STANDARD
        };
        Self::with(source_name, false, signer, None, timeouts)
    }

    /// [`Self::direct`] with other limits, so a test of a stalled body ends in a moment.
    #[cfg(test)]
    pub(crate) fn with_timeouts(source_name: &str, timeouts: HttpTimeouts) -> HttpFetch {
        Self::with(source_name, false, None, None, timeouts)
    }

    fn with(
        source_name: &str,
        https_only: bool,
        signer: Option<SigV4>,
        proxy: Option<Proxy>,
        timeouts: HttpTimeouts,
    ) -> HttpFetch {
        // ureq checks `https_only` on every call, a redirected one too.
        let redirects = if signer.is_some() { 0 } else { 10 };
        let config = Agent::config_builder()
            .proxy(proxy)
            .https_only(https_only)
            .max_redirects(redirects)
            .http_status_as_error(false)
            .timeout_connect(Some(timeouts.connect))
            .timeout_recv_response(Some(timeouts.response))
            .timeout_global(Some(timeouts.request))
            .build();
        HttpFetch {
            agent: Agent::new_with_config(config),
            signer,
            source_name: source_name.to_string(),
            timeouts,
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

    /// A GET whose body may be `size` bytes at most: it gets [`HttpTimeouts::body`] for
    /// that size, and the request [`HttpTimeouts::download`].
    fn get(&self, url: &str, size: u64) -> Result<Response<Body>> {
        let mut request = self
            .agent
            .get(url)
            .config()
            .timeout_recv_body(Some(self.timeouts.body(size)))
            .timeout_global(Some(self.timeouts.download(size)))
            .build();
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
        self.get(url, MAX_TEXT_BYTES)?
            .body_mut()
            .with_config()
            .limit(MAX_TEXT_BYTES)
            .read_to_string()
            .map_err(|e| failed(url, e))
    }

    fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<u64> {
        let mut response = self.get(url, limit)?;
        let disk = |source| SdkError::Io {
            path: dest.display().to_string(),
            source,
        };
        let mut file = File::create(dest).map_err(disk)?;
        // One byte past the limit is enough to tell the body is too long.
        let mut body = response
            .body_mut()
            .as_reader()
            .take(limit.saturating_add(1));
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
            written += read as u64;
            if written > limit {
                return Err(SdkError::longer_than(url, limit));
            }
            file.write_all(&buffer[..read]).map_err(disk)?;
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
