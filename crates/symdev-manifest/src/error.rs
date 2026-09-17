#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no symdev.toml in current directory")]
    MissingFile,
    #[error("{0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
