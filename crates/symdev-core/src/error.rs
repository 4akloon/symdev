#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not implemented: '{feature}' (unlocks at {milestone})")]
    NotImplemented {
        feature: &'static str,
        milestone: &'static str,
    },
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
