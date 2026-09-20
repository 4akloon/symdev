//! `BldInf::parse`: the `bld.inf` grammar
//! ([mmp-frontend-spec.md](../../../docs/research/mmp-frontend-spec.md) §4), over text
//! the preprocessor has already been through (`ProjectCpp`).
use std::path::PathBuf;

use crate::model::BldInf;
use crate::project::ProjectLine;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub(crate) String);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Platforms,
    Exports,
    MmpFiles,
    TestMmpFiles,
}

const USABLE_PLATFORMS: &[&str] = &["GCCE", "ARMV5", "ARMV5_ABIV2", "DEFAULT"];

fn usable_platform(tok: &str) -> bool {
    USABLE_PLATFORMS.iter().any(|p| tok.eq_ignore_ascii_case(p))
}

fn section_of(name: &str) -> Option<Section> {
    Some(match name {
        "PRJ_PLATFORMS" => Section::Platforms,
        "PRJ_EXPORTS" => Section::Exports,
        "PRJ_MMPFILES" => Section::MmpFiles,
        "PRJ_TESTMMPFILES" => Section::TestMmpFiles,
        _ => return None,
    })
}

/// Which of the two passes a line came from (§1.6): the platform pass reads
/// `PRJ_PLATFORMS` and `PRJ_EXPORTS`, the per-platform pass reads the MMP sections.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Platform,
    PerPlatform,
}

impl BldInf {
    /// The two preprocessed texts of one `bld.inf`: the platform pass (no macros) and
    /// the per-platform pass (the GCCE macro set).
    pub fn parse(platform: &str, per_platform: &str) -> Result<Self, ParseError> {
        let mut bld = Self::default();
        let mut platforms: Option<Vec<String>> = None;
        for (pass, text) in [
            (Pass::Platform, platform),
            (Pass::PerPlatform, per_platform),
        ] {
            let mut section = Section::None;
            for line in ProjectLine::records(text) {
                let name = line.directive();
                if let Some(next) = section_of(&name) {
                    section = next;
                    if section == Section::Platforms {
                        platforms.get_or_insert_with(Vec::new);
                    }
                    continue;
                }
                if name.starts_with("PRJ_") {
                    return Err(ParseError(format!(
                        "{}: unknown section: {name}",
                        line.at()
                    )));
                }
                match (pass, section) {
                    (_, Section::None) => {
                        return Err(ParseError(format!(
                            "{}: {} before any PRJ_ section",
                            line.at(),
                            line.tokens[0]
                        )));
                    }
                    (Pass::Platform, Section::Platforms) => platforms
                        .get_or_insert_with(Vec::new)
                        .extend(line.tokens.iter().cloned()),
                    (Pass::Platform, Section::Exports) => {
                        bld.exports.push(line.tokens.join(" "));
                    }
                    (Pass::PerPlatform, Section::MmpFiles) => {
                        bld.mmp_files.push(PathBuf::from(line.tokens.join(" ")));
                    }
                    (Pass::PerPlatform, Section::TestMmpFiles) => {
                        bld.test_mmp_files
                            .push(PathBuf::from(line.tokens.join(" ")));
                    }
                    _ => {}
                }
            }
        }
        if let Some(tokens) = &platforms
            && !tokens.iter().any(|t| usable_platform(t))
        {
            return Err(ParseError("PRJ_PLATFORMS has no usable platform".into()));
        }
        Ok(bld)
    }
}

#[cfg(test)]
mod tests;
