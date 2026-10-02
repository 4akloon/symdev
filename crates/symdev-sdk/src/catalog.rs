use std::collections::BTreeMap;

use crate::{ArchiveEntry, Auth, Fetch, FileFetch, Host, HttpFetch, Index, IndexPackage};
use crate::{PackageId, Result, S3Keys, SdkError, SigV4, SourceSpec, Sources};

/// The configured sources and their indexes, each fetched at most once and only when
/// something must be looked up. An `s3` source without keys is skipped, and so is one
/// whose index cannot be read; both are reported only if the id is found nowhere.
pub(crate) struct Catalog {
    sources: Sources,
    keys: BTreeMap<String, S3Keys>,
    /// By source name: the parsed index, or why it could not be read.
    indexes: BTreeMap<String, std::result::Result<Index, String>>,
}

impl Catalog {
    pub(crate) fn new(sources: Sources, keys: BTreeMap<String, S3Keys>) -> Catalog {
        Catalog {
            sources,
            keys,
            indexes: BTreeMap::new(),
        }
    }

    /// The adapter that reads `source`, or `None` for an `s3` source without keys.
    pub(crate) fn fetcher(&self, source: &SourceSpec) -> Option<Box<dyn Fetch>> {
        if source.is_file() {
            return Some(Box::new(FileFetch));
        }
        let signer = match source.auth {
            Auth::None => None,
            Auth::S3 => Some(SigV4::s3(self.keys.get(&source.name)?.clone(), "auto")),
        };
        Some(Box::new(HttpFetch::new(&source.name, signer)))
    }

    /// The first source that lists `id`, its package entry and the archive for `host`.
    pub(crate) fn find(
        &mut self,
        id: &PackageId,
        host: Host,
    ) -> Result<(SourceSpec, IndexPackage, ArchiveEntry)> {
        let mut searched = Vec::new();
        let mut problems = Vec::new();
        for source in self.sources.list.clone() {
            if self.fetcher(&source).is_none() {
                problems.push(Self::keys_hint(&source, id.kind() == "sdk"));
                continue;
            }
            searched.push(format!("`{}`", source.name));
            let index = match self.index(&source) {
                Ok(index) => index,
                Err(why) => {
                    problems.push(format!("source `{}` could not be read: {why}", source.name));
                    continue;
                }
            };
            if let Some(package) = index.find(id) {
                let entry = package.archive_for(host).ok_or_else(|| {
                    SdkError::Other(format!(
                        "{id} has no archive for {host} in source `{}`",
                        source.name
                    ))
                })?;
                return Ok((source.clone(), package.clone(), entry.clone()));
            }
        }
        let mut message = match (searched.is_empty(), self.sources.list.is_empty()) {
            (_, true) => format!(
                "{id} was not found: no package source is configured; list one in \
                 `$XDG_CONFIG_HOME/symdev/sources.toml` (`~/.config/symdev/sources.toml`)"
            ),
            (true, false) => format!("{id} was not found: no source could be searched"),
            (false, false) => format!(
                "{id} was not found in the sources searched: {}",
                searched.join(", ")
            ),
        };
        for problem in problems {
            message.push_str("; ");
            message.push_str(&problem);
        }
        Err(SdkError::Other(message))
    }

    /// Every package of every source that can be read, with the source's name; the
    /// sources that cannot are described in the second list.
    pub(crate) fn all(&mut self) -> (Vec<(String, IndexPackage)>, Vec<String>) {
        let mut packages = Vec::new();
        let mut problems = Vec::new();
        for source in self.sources.list.clone() {
            if self.fetcher(&source).is_none() {
                problems.push(Self::keys_hint(&source, false));
                continue;
            }
            match self.index(&source) {
                Ok(index) => packages.extend(
                    index
                        .packages
                        .iter()
                        .map(|p| (source.name.clone(), p.clone())),
                ),
                Err(why) => {
                    problems.push(format!("source `{}` could not be read: {why}", source.name))
                }
            }
        }
        (packages, problems)
    }

    /// Why a keyless `s3` source was skipped, and the variables that would let it in;
    /// for an SDK (`sdk`), also the way around the private source altogether.
    fn keys_hint(source: &SourceSpec, sdk: bool) -> String {
        let (key_id, secret) = S3Keys::variable_names(&source.name);
        let mut hint = format!(
            "source `{}` was skipped because its keys are not set: set {key_id} and {secret}",
            source.name
        );
        if sdk {
            hint.push_str(", or set SYMDEV_EPOCROOT to your own SDK");
        }
        hint
    }

    /// The index of `source`, fetched on first use.
    fn index(&mut self, source: &SourceSpec) -> std::result::Result<&Index, String> {
        if !self.indexes.contains_key(&source.name) {
            let fetched = match self.fetcher(source) {
                Some(fetch) => fetch
                    .text(&source.index_url())
                    .and_then(|text| Index::parse(&text, &source.name))
                    .map_err(|e| e.to_string()),
                None => Err("its keys are not set".to_string()),
            };
            self.indexes.insert(source.name.clone(), fetched);
        }
        match self.indexes.get(&source.name) {
            Some(Ok(index)) => Ok(index),
            Some(Err(why)) => Err(why.clone()),
            None => Err("its index was not kept".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::Catalog;
    use crate::{Auth, S3Keys, SourceSpec, Sources};

    /// An `s3` source is searchable exactly when it has keys; nothing is requested to
    /// find that out (`cargo test` never touches the network).
    #[test]
    fn keys_give_an_s3_source_a_fetcher() {
        let private = SourceSpec::new("private", "https://127.0.0.1:1/bucket/", Auth::S3).unwrap();
        let sources = Sources {
            list: vec![private.clone()],
        };
        let keys = BTreeMap::from([(
            "private".to_string(),
            S3Keys {
                access_key_id: "AKID".into(),
                secret_access_key: "secret".into(),
            },
        )]);
        assert!(
            Catalog::new(sources.clone(), keys)
                .fetcher(&private)
                .is_some()
        );
        assert!(
            Catalog::new(sources, BTreeMap::new())
                .fetcher(&private)
                .is_none()
        );
    }
}
