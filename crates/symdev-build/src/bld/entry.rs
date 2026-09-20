//! `PRJ_MMPFILES` and `PRJ_EXPORTS` lines (mmp-frontend-spec.md §4.3, §4.4).
use std::path::PathBuf;

use crate::bld::ParseError;
use crate::model::BldExport;
use crate::project::ProjectLine;

/// Qualifiers a `PRJ_MMPFILES` line may carry (§4.4). `BLD` is deliberately absent:
/// this SDK's parser does not know it either.
const QUALIFIERS: &[&str] = &["TIDY", "IGNORE", "BUILD_AS_ARM", "MANUAL", "SUPPORT"];

/// The external-makefile hand-off forms, which are an escape into `nmake`/GNU `make`
/// with SDK-specific goals and cannot be honoured here (§4.6).
const MAKEFILES: &[&str] = &["MAKEFILE", "NMAKEFILE", "GNUMAKEFILE"];

/// One line of a project or export section.
pub struct BldEntry;

impl BldEntry {
    /// A `PRJ_MMPFILES` line: the `.mmp` it names, or `None` when `IGNORE` discards it.
    pub fn mmp(line: &ProjectLine) -> Result<Option<PathBuf>, ParseError> {
        let [path, qualifiers @ ..] = line.tokens.as_slice() else {
            return Ok(None);
        };
        let first = path.to_ascii_uppercase();
        if let Some(kind) = MAKEFILES.iter().find(|m| **m == first) {
            let named = line.tokens.get(1).cloned().unwrap_or_default();
            return Err(ParseError(format!(
                "{}: {kind} {named}: an external makefile is built by {} with the SDK's own \
                 build stages as goals; symdev has no makefile stage, so move what it does \
                 into the project or build it yourself before `symdev build`",
                line.at(),
                if first == "GNUMAKEFILE" {
                    "GNU make"
                } else {
                    "nmake"
                }
            )));
        }
        let mut ignore = false;
        for qualifier in qualifiers {
            let name = qualifier.to_ascii_uppercase();
            if !QUALIFIERS.contains(&name.as_str()) {
                return Err(ParseError(format!(
                    "{}: unknown qualifier: {qualifier}",
                    line.at()
                )));
            }
            ignore |= name == "IGNORE";
        }
        Ok((!ignore).then(|| PathBuf::from(path)))
    }

    /// A `PRJ_EXPORTS` line: `<source> [<destination>]`, or `:zip <archive> [<dir>]`.
    pub fn export(line: &ProjectLine) -> Result<BldExport, ParseError> {
        let tokens = &line.tokens;
        if let Some(kind) = tokens[0].strip_prefix(':') {
            if !kind.eq_ignore_ascii_case("zip") {
                return Err(ParseError(format!(
                    "{}: unknown archive type: :{kind}",
                    line.at()
                )));
            }
            let source = tokens
                .get(1)
                .cloned()
                .ok_or_else(|| ParseError(format!("{}: :zip without an archive", line.at())))?;
            return Ok(BldExport {
                source,
                dest: tokens.get(2).cloned(),
                zip: true,
            });
        }
        if let Some(extra) = tokens.get(2) {
            return Err(ParseError(format!(
                "{}: {extra}: only :zip takes a third token",
                line.at()
            )));
        }
        Ok(BldExport {
            source: tokens[0].clone(),
            dest: tokens.get(1).cloned(),
            zip: false,
        })
    }
}
