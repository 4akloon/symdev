use crate::{Auth, Result, SdkError, SourceSpec};

/// Every source, in the order an id is looked up: the built-in source first (unless
/// `sources.toml` says `builtin = false`), then the listed ones in file order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sources {
    pub list: Vec<SourceSpec>,
}

/// `sources.toml` as written; unknown keys are refused, so a typo (`auht = "s3"`) cannot
/// silently turn signing off.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesFile {
    builtin: Option<bool>,
    #[serde(rename = "source", default)]
    sources: Vec<SourceEntry>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEntry {
    name: String,
    url: String,
    #[serde(default)]
    auth: Auth,
}

impl Sources {
    /// `text` is the contents of `sources.toml` (`None` if the file does not exist);
    /// `builtin` is the built-in source, if this symdev has one. Names must be unique.
    pub fn parse(
        text: Option<&str>,
        path_for_errors: &str,
        builtin: Option<&SourceSpec>,
    ) -> Result<Sources> {
        let bad = |detail: String| SdkError::BadSources {
            path: path_for_errors.to_string(),
            detail,
        };
        let file: SourcesFile = match text {
            Some(text) => toml::from_str(text).map_err(|e| bad(e.to_string()))?,
            None => SourcesFile {
                builtin: None,
                sources: vec![],
            },
        };
        let mut list: Vec<SourceSpec> = Vec::new();
        if file.builtin.unwrap_or(true) {
            list.extend(builtin.cloned());
        }
        for entry in file.sources {
            let spec = SourceSpec::new(&entry.name, &entry.url, entry.auth)
                .map_err(|e| bad(e.to_string()))?;
            if list.iter().any(|s| s.name == spec.name) {
                return Err(bad(format!(
                    "source `{}` is listed twice; names must be unique",
                    spec.name
                )));
            }
            list.push(spec);
        }
        Ok(Sources { list })
    }
}

#[cfg(test)]
mod tests;
