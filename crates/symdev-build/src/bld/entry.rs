//! `PRJ_MMPFILES` and `PRJ_EXPORTS` lines (mmp-frontend-spec.md §4.3, §4.4).
use std::path::PathBuf;

use crate::bld::ParseError;
use crate::model::BldExport;
use crate::project::ProjectLine;

/// Qualifiers a `PRJ_MMPFILES` line may carry (§4.4). `BLD` is deliberately absent:
/// this SDK's parser does not know it either.
const QUALIFIERS: &[&str] = &["TIDY", "IGNORE", "BUILD_AS_ARM", "MANUAL", "SUPPORT"];

/// The external-makefile hand-off forms, which are an escape into `nmake`/GNU `make`
/// with SDK-specific goals (§4.6). symdev runs no makefile: the line is skipped with a
/// warning, and what the makefile would have produced is declared in `symdev.toml`.
const MAKEFILES: &[&str] = &["MAKEFILE", "NMAKEFILE", "GNUMAKEFILE"];

/// One line of a project or export section.
pub struct BldEntry;

impl BldEntry {
    /// A `PRJ_MMPFILES` line: the `.mmp` it names, or `None` when `IGNORE` discards it
    /// or the line hands off to a makefile (then `warnings` says so).
    pub fn mmp(
        line: &ProjectLine,
        warnings: &mut Vec<String>,
    ) -> Result<Option<PathBuf>, ParseError> {
        let [path, qualifiers @ ..] = line.tokens.as_slice() else {
            return Ok(None);
        };
        let first = path.to_ascii_uppercase();
        if MAKEFILES.contains(&first.as_str()) {
            let named = line.tokens.get(1).cloned().unwrap_or_default();
            warnings.push(format!(
                "{}: {path} {named}: symdev does not run makefiles, so this line is skipped; \
                 declare what {named} produces in symdev.toml ([[icons]] for a mifconv call, \
                 [[install]] for a file it copies)",
                line.at()
            ));
            return Ok(None);
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
