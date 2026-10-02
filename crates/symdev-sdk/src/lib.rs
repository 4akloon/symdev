//! The toolchain manager: package ids, repository indexes, sources and the installed
//! packages under `SYMDEV_HOME` (design: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`).

mod amz_date;
mod error;
mod fetch;
mod keys;
mod sigv4;

pub use amz_date::AmzDate;
pub use error::{Result, SdkError};
pub use fetch::Fetch;
pub use keys::S3Keys;
pub use sigv4::SigV4;
