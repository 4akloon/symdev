//! `Evidence`: why a test failed, as the report shows it.
use alloc::string::String;

/// A failed test's text. Any error the report can show converts into it, so `?` works in
/// a test on a `symbian_std` call.
pub struct Evidence(String);

impl<E: symbian_std::test_report::Evidence> From<E> for Evidence {
    fn from(e: E) -> Self {
        Self(e.shown())
    }
}

impl Evidence {
    /// A failure described in words.
    pub fn msg(what: &str) -> Self {
        Self(String::from(what))
    }

    /// The text the report carries.
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// `Ok(())` when `ok`, else a failure that says `what` did not hold.
pub fn ensure(ok: bool, what: &str) -> Result<(), Evidence> {
    match ok {
        true => Ok(()),
        false => Err(Evidence::msg(what)),
    }
}
