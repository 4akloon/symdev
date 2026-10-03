//! `HttpTimeouts`: how long [`crate::HttpFetch`] waits, phase by phase (spec §4).
use std::time::Duration;

/// The limits of one HTTP request. ureq 3.4.2 has no idle timeout, only per-phase totals
/// (`timeout_recv_body` is "max total duration for receiving the response body", its
/// budget not restarted per read), so a download's body is bounded by its size at a
/// minimum rate instead: a server that sends the headers and then stalls is given up on
/// once a link of `body_rate` would have delivered the whole body.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HttpTimeouts {
    /// Connecting, TLS handshake included.
    pub(crate) connect: Duration,
    /// The server's answer: its response headers.
    pub(crate) response: Duration,
    /// A whole request of any kind; a download that needs longer at `body_rate` gets
    /// longer ([`Self::download`]).
    pub(crate) request: Duration,
    /// A body's allowance before its size counts.
    pub(crate) body_base: Duration,
    /// The slowest link a download must keep up, in bytes per second.
    pub(crate) body_rate: u64,
}

impl HttpTimeouts {
    /// 30 s to connect, 60 s for the answer, an hour per request; a body gets 60 s plus
    /// its size at 16 KiB/s (`gcce;12.1.0`, 67 MB: 70 minutes, more than the hour).
    pub(crate) const STANDARD: HttpTimeouts = HttpTimeouts {
        connect: Duration::from_secs(30),
        response: Duration::from_secs(60),
        request: Duration::from_secs(60 * 60),
        body_base: Duration::from_secs(60),
        body_rate: 16 * 1024,
    };

    /// The longest the body of a `size`-byte download may take: `body_base` plus `size`
    /// at `body_rate`.
    pub(crate) fn body(&self, size: u64) -> Duration {
        let rate = self.body_rate.max(1);
        let rest = u128::from(size % rate) * 1_000_000_000 / u128::from(rate);
        let nanos = u32::try_from(rest).unwrap_or(999_999_999);
        self.body_base
            .saturating_add(Duration::new(size / rate, nanos))
    }

    /// The longest a whole `size`-byte download may take: `request`, or, when the body
    /// alone may take longer, connecting, the answer and the body together.
    pub(crate) fn download(&self, size: u64) -> Duration {
        let whole = self.connect.saturating_add(self.response);
        self.request.max(whole.saturating_add(self.body(size)))
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::HttpTimeouts;

    const S: HttpTimeouts = HttpTimeouts::STANDARD;

    #[test]
    fn a_body_gets_a_minute_plus_its_size_at_16_kib_a_second() {
        assert_eq!(S.body(0), Duration::from_secs(60));
        assert_eq!(S.body(16 * 1024 * 100), Duration::from_secs(160));
        assert_eq!(S.body(8 * 1024), Duration::from_millis(60_500));
    }

    /// The published `gcce;12.1.0` (spec §13) through a 19 KB/s link, which symdev 0.1.0's
    /// hour let through (review 0.2.0, minor 1): 3 551 s of body, inside its 4 178 s.
    #[test]
    fn gcce_gets_through_a_link_that_0_1_0_allowed() {
        let gcce = 67_463_658;
        assert_eq!(S.body(gcce).as_secs(), 60 + gcce / (16 * 1024));
        assert!(S.body(gcce) > Duration::from_secs(gcce / 19_000 + 1));
        assert_eq!(S.download(gcce), S.connect + S.response + S.body(gcce));
    }

    /// 200 MB at exactly 16 KiB/s is 12 207 s: the hour would cut a download that keeps up.
    #[test]
    fn a_body_longer_than_the_hour_extends_the_request() {
        let size = 200_000_000;
        let whole = S.connect + S.response + S.body(size);
        assert!(whole > Duration::from_secs(3600));
        assert_eq!(S.download(size), whole);
    }

    #[test]
    fn a_huge_size_saturates_instead_of_overflowing() {
        assert!(S.body(u64::MAX) > Duration::from_secs(u64::MAX / (16 * 1024)));
        S.download(u64::MAX);
    }
}
