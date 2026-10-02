use std::collections::BTreeSet;

use crate::url::relative_url_problem;
use crate::{IndexPackage, PackageId, Result, SdkError};

/// The only index schema this symdev reads and writes.
const SCHEMA: u32 = 1;

/// A source's `index.toml`: every package the source publishes, with one archive per host.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Index {
    pub schema: u32,
    #[serde(rename = "package", default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<IndexPackage>,
}

/// Read before the rest, so a newer index is reported as newer, not as malformed.
#[derive(serde::Deserialize)]
struct SchemaOnly {
    schema: Option<u32>,
}

impl Index {
    /// Schema 1, no packages: what a new bucket starts from.
    pub fn empty() -> Self {
        Index {
            schema: SCHEMA,
            packages: vec![],
        }
    }

    /// Parses and checks `text`, the `index.toml` of source `source_name`: the schema,
    /// unique ids, at least one archive per package and one per host, SHA-256 as 64
    /// lowercase hex digits, and URLs that stay under the index's directory.
    pub fn parse(text: &str, source_name: &str) -> Result<Index> {
        let bad = |detail: String| SdkError::BadIndex {
            source_name: source_name.to_string(),
            detail,
        };
        let head: SchemaOnly = toml::from_str(text).map_err(|e| bad(e.to_string()))?;
        match head.schema {
            None => return Err(bad("missing `schema`".into())),
            Some(found) if found > SCHEMA => {
                return Err(SdkError::UnknownSchema {
                    source_name: source_name.to_string(),
                    found,
                });
            }
            Some(found) if found < SCHEMA => {
                return Err(bad(format!("`schema = {found}` is not a schema version")));
            }
            Some(_) => {}
        }
        let index: Index = toml::from_str(text).map_err(|e| bad(e.to_string()))?;
        let mut seen = BTreeSet::new();
        for p in &index.packages {
            if !seen.insert(&p.id) {
                return Err(bad(format!("package {} is listed twice", p.id)));
            }
            Self::package_problem(p).map_or(Ok(()), |d| Err(bad(d)))?;
        }
        Ok(index)
    }

    fn package_problem(p: &IndexPackage) -> Option<String> {
        if p.archives.is_empty() {
            return Some(format!("package {} has no archive", p.id));
        }
        for (n, a) in p.archives.iter().enumerate() {
            if p.archives[..n].iter().any(|b| b.host == a.host) {
                return Some(format!(
                    "package {} lists two archives for {}",
                    p.id, a.host
                ));
            }
            let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
            if a.sha256.len() != 64 || !a.sha256.chars().all(hex) {
                return Some(format!(
                    "package {}: sha256 `{}` is not 64 lowercase hex digits",
                    p.id, a.sha256
                ));
            }
        }
        let urls = p.archives.iter().map(|a| &a.url).chain(&p.source_code);
        for url in urls {
            if let Some(reason) = relative_url_problem(url) {
                return Some(format!("package {}: URL `{url}` {reason}", p.id));
            }
        }
        None
    }

    /// The index as TOML, packages sorted by id, so a rewrite changes only what changed.
    pub fn to_toml(&self) -> Result<String> {
        let mut sorted = self.clone();
        sorted.packages.sort_by(|a, b| a.id.cmp(&b.id));
        toml::to_string(&sorted)
            .map_err(|e| SdkError::Other(format!("cannot write the index: {e}")))
    }

    pub fn find(&self, id: &PackageId) -> Option<&IndexPackage> {
        self.packages.iter().find(|p| &p.id == id)
    }

    /// Adds a newly published package. An id already present is refused: ids are
    /// immutable, so a rebuild must be published under a new version.
    pub fn insert(&mut self, p: IndexPackage) -> Result<()> {
        if self.find(&p.id).is_some() {
            return Err(SdkError::Other(format!(
                "{} is already published; a rebuild is a new version",
                p.id
            )));
        }
        if let Some(detail) = Self::package_problem(&p) {
            return Err(SdkError::Other(format!("cannot publish: {detail}")));
        }
        self.packages.push(p);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
