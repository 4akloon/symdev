use std::path::Path;

use crate::Result;

/// Where a source's bytes come from: `file://` directories (tests, local mirrors) or
/// HTTP(S). Implementations are the only code that touches a source.
pub trait Fetch {
    /// The whole body of `url` as UTF-8 text (an `index.toml`).
    fn text(&self, url: &str) -> Result<String>;

    /// Streams `url` into `dest` (created or truncated); returns the bytes written. More
    /// than `limit` bytes (the index's `size`) is an error naming the URL and the limit.
    fn download(&self, url: &str, dest: &Path, limit: u64) -> Result<u64>;
}
