use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use crate::catalog::Catalog;
use crate::{Host, IndexPackage, PackageId, Receipt, Result, S3Keys, SdkError, SdkHome, Sources};

/// Installs what a build is missing: looks ids up in the sources (first source wins),
/// installs their dependencies first, and writes one progress line per install. Nothing
/// is fetched, not even an index, while every requested id is installed.
pub struct SdkManager<'w> {
    home: SdkHome,
    catalog: Catalog,
    offline: bool,
    host: Host,
    progress: &'w mut dyn Write,
}

impl<'w> SdkManager<'w> {
    /// `keys` holds the S3 keys of the `s3` sources, by source name; an `s3` source
    /// without keys is skipped. `offline` forbids reading any source.
    pub fn new(
        home: SdkHome,
        sources: Sources,
        keys: BTreeMap<String, S3Keys>,
        offline: bool,
        progress: &'w mut dyn Write,
    ) -> Result<Self> {
        let host = Host::current().ok_or_else(|| {
            SdkError::Other(format!(
                "toolchain packages exist only for {}; set the SYMDEV_* toolchain variables \
                 to your own toolchain and SDK",
                Host::X86_64Linux
            ))
        })?;
        Ok(SdkManager {
            home,
            catalog: Catalog::new(sources, keys),
            offline,
            host,
            progress,
        })
    }

    /// Installs every id that is not installed yet (and its dependencies), and returns
    /// the receipts of `ids` in order.
    pub fn ensure(&mut self, ids: &[PackageId]) -> Result<Vec<Receipt>> {
        if self.offline {
            let mut missing = Vec::new();
            for id in ids {
                if self.home.installed(id)?.is_none() {
                    missing.push(id.clone());
                }
            }
            if !missing.is_empty() {
                return Err(Self::offline_error(&missing));
            }
        }
        let mut receipts = Vec::new();
        for id in ids {
            receipts.push(self.ensure_one(id, &mut Vec::new())?);
        }
        Ok(receipts)
    }

    /// `id`, installed after its dependencies; `chain` holds the ids that wait for it.
    fn ensure_one(&mut self, id: &PackageId, chain: &mut Vec<PackageId>) -> Result<Receipt> {
        if let Some(receipt) = self.home.installed(id)? {
            return Ok(receipt);
        }
        if self.offline {
            return Err(Self::offline_error(std::slice::from_ref(id)));
        }
        if chain.contains(id) {
            let cycle: Vec<_> = chain.iter().map(PackageId::as_str).collect();
            return Err(SdkError::Other(format!(
                "{id} depends on itself through a dependency cycle ({} -> {id}); the source's \
                 index is damaged",
                cycle.join(" -> ")
            )));
        }
        let (source, package, entry) = self.catalog.find(id, self.host)?;
        chain.push(id.clone());
        for dependency in &package.depends {
            self.ensure_one(dependency, chain)?;
        }
        chain.pop();
        let fetch = self
            .catalog
            .fetcher(&source)
            .ok_or_else(|| SdkError::Other(format!("source `{}` lost its keys", source.name)))?;
        // Progress is informational: a closed stderr must not stop an install.
        let _ = writeln!(
            self.progress,
            "installing {id} ({:.1} MB) from {}…",
            entry.size as f64 / 1_000_000.0,
            source.name
        );
        self.home.install(id, &source, fetch.as_ref(), &entry)
    }

    fn offline_error(missing: &[PackageId]) -> SdkError {
        let names: Vec<_> = missing.iter().map(PackageId::as_str).collect();
        let words: Vec<_> = missing.iter().map(PackageId::shell_word).collect();
        let (verb, it) = match missing {
            [_] => ("is", "it"),
            _ => ("are", "them"),
        };
        SdkError::Other(format!(
            "{} {verb} not installed and --offline forbids downloading {it}; run `symdev sdk \
             install {}`",
            names.join(" and "),
            words.join(" ")
        ))
    }

    /// Every package that the sources offer for this host, each id once, from the first
    /// source that lists it. A source that is skipped or cannot be read is reported as a
    /// warning on the progress output.
    pub fn available(&mut self) -> Result<Vec<(String, IndexPackage)>> {
        if self.offline {
            return Err(SdkError::Other(
                "--offline forbids reading the sources' indexes; drop it to list the \
                 available packages"
                    .into(),
            ));
        }
        let (packages, problems) = self.catalog.all();
        for problem in problems {
            let _ = writeln!(self.progress, "warning: {problem}");
        }
        let mut seen = BTreeSet::new();
        Ok(packages
            .into_iter()
            .filter(|(_, p)| p.archive_for(self.host).is_some() && seen.insert(p.id.clone()))
            .collect())
    }

    pub fn home(&self) -> &SdkHome {
        &self.home
    }
}

#[cfg(test)]
mod tests;
