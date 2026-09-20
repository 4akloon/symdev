//! `[[icons]]`: one icon container per entry — exactly one `mifconv` call.
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Error, Result};
use crate::install::InstallFile;

/// One `[[icons]]` entry, the shape of a `mifconv` invocation: the output `.mif`, an
/// optional generated header, and the sources in order, each with its own depth. A
/// `.bmp` source makes `mifconv` write a sibling `.mbm` next to the `.mif`, and so does
/// symdev. `[symbian] icon` remains the shorthand for one SVG in `<app>_aif.mif`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconContainer {
    /// Install destination of the `.mif`, normalised to `!:\…`; the build writes
    /// `build/<file name>`.
    pub dest: String,
    /// `/H`: the `.mbg` file name, written to `build/` so the sources can include it.
    pub header: Option<String>,
    pub sources: Vec<IconSource>,
}

/// One source of a container: `/OPT [/A] file`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconSource {
    /// `.svg` or `.bmp`, relative to the project root.
    pub file: PathBuf,
    /// `mifconv`'s `DEPTH[,MASK]`, as in `c24` or `c32,8`.
    pub depth: String,
    /// `/A`.
    pub animated: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawIconContainer {
    pub(crate) dest: String,
    pub(crate) header: Option<String>,
    /// The depth of every source that gives none of its own.
    pub(crate) depth: Option<String>,
    #[serde(default)]
    pub(crate) sources: Vec<RawIconSource>,
}

/// `"gfx/a.bmp"` or `{ file = "gfx/a.svg", depth = "c32,8", animated = true }`.
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum RawIconSource {
    File(PathBuf),
    Detailed(RawIconSourceDetail),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawIconSourceDetail {
    pub(crate) file: PathBuf,
    pub(crate) depth: Option<String>,
    pub(crate) animated: Option<bool>,
}

impl IconContainer {
    pub(crate) fn validate_all(raw: Vec<RawIconContainer>) -> Result<Vec<Self>> {
        let mut out: Vec<Self> = Vec::with_capacity(raw.len());
        for raw in raw {
            let container = Self::validate(raw)?;
            let name = container.mif_name();
            if out.iter().any(|c| c.mif_name().eq_ignore_ascii_case(name)) {
                return Err(Error::Invalid(format!(
                    "icons: two containers build `{name}`; every [[icons]] dest needs its \
                     own file name, since each is written to build/"
                )));
            }
            if let Some(header) = &container.header
                && out.iter().any(|c| c.header.as_deref() == Some(header))
            {
                return Err(Error::Invalid(format!(
                    "icons: two containers write the header `{header}`"
                )));
            }
            out.push(container);
        }
        Ok(out)
    }

    fn validate(raw: RawIconContainer) -> Result<Self> {
        let dest =
            InstallFile::dest(&raw.dest).map_err(|e| Error::Invalid(format!("icons.{e}")))?;
        let name = dest.rsplit('\\').next().unwrap_or(&dest);
        if !name.to_ascii_lowercase().ends_with(".mif") {
            return Err(Error::Invalid(format!(
                "icons.dest `{}` must name a .mif file: mifconv writes <stem>.mif whatever \
                 extension it is given, and a .bmp source adds <stem>.mbm next to it",
                raw.dest
            )));
        }
        let header = match raw.header {
            None => None,
            Some(h) if h.is_empty() || h.contains(['/', '\\']) => {
                return Err(Error::Invalid(format!(
                    "icons.header `{h}` must be a bare file name, as in `games.mbg`; it is \
                     written to build/"
                )));
            }
            Some(h) => Some(h),
        };
        if raw.sources.is_empty() {
            return Err(Error::Invalid(format!(
                "icons `{}` lists no sources",
                raw.dest
            )));
        }
        let sources = raw
            .sources
            .into_iter()
            .map(|s| IconSource::validate(s, raw.depth.as_deref(), &raw.dest))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            dest,
            header,
            sources,
        })
    }

    /// The `.mif` file name, which is also its name in `build/`.
    pub fn mif_name(&self) -> &str {
        self.dest.rsplit('\\').next().unwrap_or(&self.dest)
    }
}

impl IconSource {
    fn validate(raw: RawIconSource, default_depth: Option<&str>, dest: &str) -> Result<Self> {
        let (file, depth, animated) = match raw {
            RawIconSource::File(file) => (file, None, false),
            RawIconSource::Detailed(d) => (d.file, d.depth, d.animated.unwrap_or(false)),
        };
        let shown = file.display().to_string();
        if shown.is_empty() || file.is_absolute() {
            return Err(Error::Invalid(format!(
                "icons `{dest}`: source `{shown}` must be a path relative to the project root"
            )));
        }
        let extension = file
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if !matches!(extension.as_deref(), Some("svg" | "bmp")) {
            return Err(Error::Invalid(format!(
                "icons `{dest}`: source `{shown}` must be a .svg or a .bmp, the two kinds \
                 mifconv takes"
            )));
        }
        let Some(depth) = depth.as_deref().or(default_depth) else {
            return Err(Error::Invalid(format!(
                "icons `{dest}`: source `{shown}` has no depth; give it one (`depth = \"c24\"`) \
                 or set the container's default depth"
            )));
        };
        if depth.is_empty() {
            return Err(Error::Invalid(format!(
                "icons `{dest}`: source `{shown}` has an empty depth"
            )));
        }
        Ok(Self {
            file,
            depth: depth.to_string(),
            animated,
        })
    }
}

#[cfg(test)]
mod tests;
