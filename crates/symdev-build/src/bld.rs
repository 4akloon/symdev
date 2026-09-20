//! `BldInf::parse`: the `bld.inf` grammar
//! ([mmp-frontend-spec.md](../../../docs/research/mmp-frontend-spec.md) §4), over text
//! the preprocessor has already been through (`ProjectCpp`).
mod entry;
mod platforms;

use crate::model::BldInf;
use crate::project::ProjectLine;

use entry::BldEntry;
use platforms::BldPlatforms;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub(crate) String);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Platforms,
    Exports,
    TestExports,
    MmpFiles,
    TestMmpFiles,
    Extensions,
}

fn section_of(name: &str) -> Option<Section> {
    Some(match name {
        "PRJ_PLATFORMS" => Section::Platforms,
        "PRJ_EXPORTS" => Section::Exports,
        "PRJ_TESTEXPORTS" => Section::TestExports,
        "PRJ_MMPFILES" => Section::MmpFiles,
        "PRJ_TESTMMPFILES" => Section::TestMmpFiles,
        "PRJ_EXTENSIONS" | "PRJ_TESTEXTENSIONS" => Section::Extensions,
        _ => return None,
    })
}

/// Which of the two passes a line came from (§1.6): the platform pass reads
/// `PRJ_PLATFORMS` and the export sections, the per-platform pass the rest.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Platform,
    PerPlatform,
}

/// One `bld.inf`, both passes, as it accumulates.
#[derive(Default)]
struct BldParser {
    bld: BldInf,
    platform_names: Vec<String>,
    defaulted: bool,
}

impl BldParser {
    fn line(&mut self, pass: Pass, section: Section, line: &ProjectLine) -> Result<(), ParseError> {
        match (pass, section) {
            (_, Section::None) => {
                return Err(ParseError(format!(
                    "{}: {} before any PRJ_ section",
                    line.at(),
                    line.tokens[0]
                )));
            }
            // §4.5: every `START EXTENSION` names a makefile template, and this SDK ships
            // no `epoc32/tools/makefile_templates/` at all.
            (Pass::PerPlatform, Section::Extensions) => {
                if line.directive() == "START" {
                    let named = line.tokens.get(2).cloned().unwrap_or_default();
                    return Err(ParseError(format!(
                        "{}: START EXTENSION {named}: extension templates are a makefile \
                         hand-off; this SDK ships no epoc32/tools/makefile_templates and \
                         symdev has no makefile stage",
                        line.at()
                    )));
                }
            }
            (Pass::Platform, Section::Platforms) => {
                BldPlatforms::line(&mut self.platform_names, &mut self.defaulted, line)?;
            }
            (Pass::Platform, Section::Exports) => {
                self.bld.exports.push(BldEntry::export(line)?);
            }
            (Pass::Platform, Section::TestExports) => {
                self.bld.test_exports.push(BldEntry::export(line)?);
            }
            (Pass::PerPlatform, Section::MmpFiles) => {
                if let Some(path) = BldEntry::mmp(line)? {
                    self.bld.mmp_files.push(path);
                }
            }
            (Pass::PerPlatform, Section::TestMmpFiles) => {
                if let Some(path) = BldEntry::mmp(line)? {
                    self.bld.test_mmp_files.push(path);
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl BldInf {
    /// The two preprocessed texts of one `bld.inf`: the platform pass (no macros) and
    /// the per-platform pass (the GCCE macro set).
    pub fn parse(platform: &str, per_platform: &str) -> Result<Self, ParseError> {
        let mut parser = BldParser::default();
        for (pass, text) in [
            (Pass::Platform, platform),
            (Pass::PerPlatform, per_platform),
        ] {
            let mut section = Section::None;
            for line in ProjectLine::records(text) {
                let name = line.directive();
                if let Some(next) = section_of(&name) {
                    if line.tokens.len() > 1 {
                        return Err(ParseError(format!(
                            "{}: {name} takes the whole line",
                            line.at()
                        )));
                    }
                    section = next;
                    continue;
                }
                if name.starts_with("PRJ_") {
                    return Err(ParseError(format!(
                        "{}: unknown section: {name}",
                        line.at()
                    )));
                }
                parser.line(pass, section, &line)?;
            }
        }
        let mut bld = parser.bld;
        bld.platforms = BldPlatforms::finish(parser.platform_names);
        Ok(bld)
    }
}

#[cfg(test)]
mod tests;
