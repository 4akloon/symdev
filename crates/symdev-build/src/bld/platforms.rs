//! `PRJ_PLATFORMS` (mmp-frontend-spec.md §4.2).
use crate::bld::ParseError;
use crate::project::ProjectLine;

/// The default list on this SDK: no RVCT compiler is installed, so `ARMV5` is not in it.
const DEFAULT: &[&str] = &["WINSCW", "GCCXML", "EDG"];
const BASEDEFAULT: &[&str] = &["ARM4", "ARM4T", "WINSCW", "GCCXML", "EDG"];

/// The platform list one `bld.inf` declares.
pub struct BldPlatforms;

impl BldPlatforms {
    /// Expand one `PRJ_PLATFORMS` line into `names`. `WINC` is dropped, `DEFAULT` and
    /// `BASEDEFAULT` expand, `-<name>` removes a name a default put there, and any
    /// `VC…` name is fatal.
    pub fn line(
        names: &mut Vec<String>,
        defaulted: &mut bool,
        line: &ProjectLine,
    ) -> Result<(), ParseError> {
        for token in &line.tokens {
            let name = token.to_ascii_uppercase();
            if let Some(dropped) = name.strip_prefix('-') {
                if !*defaulted {
                    return Err(ParseError(format!(
                        "{}: PRJ_PLATFORMS {token}: a platform can only be removed after \
                         DEFAULT or BASEDEFAULT",
                        line.at()
                    )));
                }
                names.retain(|n| n != dropped);
                continue;
            }
            match name.as_str() {
                "WINC" => {}
                "DEFAULT" => {
                    *defaulted = true;
                    names.extend(DEFAULT.iter().map(|p| (*p).to_string()));
                }
                "BASEDEFAULT" => {
                    *defaulted = true;
                    names.extend(BASEDEFAULT.iter().map(|p| (*p).to_string()));
                }
                _ if name.starts_with("VC") => {
                    return Err(ParseError(format!(
                        "{}: PRJ_PLATFORMS {token}: the Visual C++ pseudo-platforms are not \
                         supported",
                        line.at()
                    )));
                }
                _ => {
                    if name.starts_with("TOOLS") {
                        names.push("CWTOOLS".to_string());
                    }
                    names.push(name);
                }
            }
        }
        Ok(())
    }

    /// GCCE is an optional platform: the SDK appends it to every component's list
    /// because `epoc32/tools/compilation_config/gcce.mk` exists, so it is always
    /// buildable whatever `PRJ_PLATFORMS` says.
    pub fn finish(mut names: Vec<String>) -> Vec<String> {
        if !names.iter().any(|n| n == "GCCE") {
            names.push("GCCE".to_string());
        }
        names.dedup();
        names
    }
}
