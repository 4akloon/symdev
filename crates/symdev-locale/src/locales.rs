//! `Locales`: a project's `locales/` directory, checked as a whole.
use std::path::Path;

use crate::file::RESERVED;
use crate::{Error, Language, Table};

/// The first resource index a string can have: index 1 is `RSS_SIGNATURE`.
pub const FIRST_INDEX: u16 = 2;

/// The fallback file and every language variant, in file-name order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Locales {
    pub default: Table,
    pub variants: Vec<(Language, Table)>,
}

impl Locales {
    /// Reads `dir`. `Ok(None)` when it does not exist: a project without `locales/` has
    /// no localised strings and nothing is compiled for it.
    pub fn load(dir: &Path) -> Result<Option<Locales>, Error> {
        if !dir.is_dir() {
            return Ok(None);
        }
        let read = |path: &Path| {
            std::fs::read_to_string(path).map_err(|e| Error(format!("{}: {e}", path.display())))
        };
        let default_path = dir.join("default.toml");
        if !default_path.is_file() {
            return Err(Error(format!(
                "{} has no default.toml: it is the fallback a device gets when no file \
                 matches its language, and every other file is checked against it",
                dir.display()
            )));
        }
        let default = Table::parse("default.toml", &read(&default_path)?)?;
        if let Some(key) = default
            .entries
            .keys()
            .find(|k| RESERVED.contains(&k.as_str()))
        {
            return Err(Error(format!(
                "default.toml: `{key}` belongs in symdev.toml's [ui]; a locales file \
                 translates it, the default file does not repeat it"
            )));
        }
        let mut names: Vec<_> = std::fs::read_dir(dir)
            .map_err(|e| Error(format!("{}: {e}", dir.display())))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "toml"))
            .collect();
        names.sort();
        let mut variants = Vec::new();
        for path in names {
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if stem == "default" {
                continue;
            }
            let file = format!("{stem}.toml");
            let language = Language::named(stem).ok_or_else(|| {
                Error(format!(
                    "{file}: `{stem}` is not a language this SDK knows; a locales file is \
                     named after a TLanguage, e.g. english.toml, french.toml, ukrainian.toml"
                ))
            })?;
            let table = Table::parse(&file, &read(&path)?)?;
            same_keys(&file, &default, &table)?;
            variants.push((language, table));
        }
        Ok(Some(Locales { default, variants }))
    }

    /// The string keys, in byte order: the order they are compiled in.
    pub fn keys(&self) -> Vec<&str> {
        self.default.strings().collect()
    }

    /// The resource index `key` is compiled at, or `None` if it is not a string key.
    pub fn index(&self, key: &str) -> Option<u16> {
        let position = self.keys().iter().position(|k| *k == key)?;
        u16::try_from(position).ok()?.checked_add(FIRST_INDEX)
    }
}

/// A variant must translate every string of the default and nothing else.
fn same_keys(file: &str, default: &Table, variant: &Table) -> Result<(), Error> {
    if let Some(key) = default
        .strings()
        .find(|k| !variant.entries.contains_key(*k))
    {
        return Err(Error(format!(
            "{file} has no `{key}`: every string in default.toml needs a translation"
        )));
    }
    if let Some(key) = variant
        .strings()
        .find(|k| !default.entries.contains_key(*k))
    {
        return Err(Error(format!(
            "{file} has `{key}`, which default.toml does not: add it there first"
        )));
    }
    Ok(())
}
