//! The toolchain manager: package ids, repository indexes, sources and the installed
//! packages under `SYMDEV_HOME` (design: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`).

mod error;
mod fetch;
mod host;
mod index;
mod index_package;
mod keys;
mod package_id;
mod reproducible;
mod source;
mod sources;
mod tar_gz;
mod url;

pub use error::{Result, SdkError};
pub use fetch::Fetch;
pub use host::Host;
pub use index::Index;
pub use index_package::{ArchiveEntry, IndexPackage};
pub use keys::S3Keys;
pub use package_id::PackageId;
pub use reproducible::ReproducibleTarGz;
pub use source::{Auth, SourceSpec};
pub use sources::Sources;
pub use tar_gz::TarGz;
pub use url::resolve_url;
