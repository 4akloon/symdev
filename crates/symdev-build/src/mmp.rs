//! `Mmp::parse`: the `.mmp` grammar
//! ([mmp-frontend-spec.md](../../../docs/research/mmp-frontend-spec.md) §5), over text
//! the preprocessor has already been through (`ProjectCpp`).
use crate::bld::ParseError;
use crate::model::{Mmp, MmpResource};
use crate::project::ProjectLine;

fn parse_uid_token(tok: &str) -> Result<u32, ParseError> {
    let (radix, digits) =
        if let Some(hex) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
            (16, hex)
        } else {
            (10, tok)
        };
    u32::from_str_radix(digits, radix).map_err(|_| ParseError(format!("invalid UID: {tok}")))
}

/// The `.mmp` fields as they accumulate, one line at a time.
#[derive(Default)]
struct MmpParser {
    mmp: Mmp,
    block: Option<MmpResource>,
}

impl MmpParser {
    /// A line inside `START RESOURCE … END`.
    fn resource_line(&mut self, line: &ProjectLine) -> Result<(), ParseError> {
        let Some(block) = self.block.as_mut() else {
            return Ok(());
        };
        let args = line.args();
        match line.directive().as_str() {
            "END" => {
                if let Some(block) = self.block.take() {
                    self.mmp.resource.push(block);
                }
            }
            "HEADER" => block.header = true,
            "TARGETPATH" => block.targetpath = args.first().cloned(),
            "LANG" => block.lang.extend(args.iter().cloned()),
            other => {
                return Err(ParseError(format!(
                    "{}: TODO: START RESOURCE directive {other} (not observed)",
                    line.at()
                )));
            }
        }
        Ok(())
    }

    fn line(&mut self, line: &ProjectLine) -> Result<(), ParseError> {
        if self.block.is_some() {
            return self.resource_line(line);
        }
        let args = line.args();
        let m = &mut self.mmp;
        match line.directive().as_str() {
            "START" => {
                let kind = args.first().map(String::as_str).unwrap_or("");
                if !kind.eq_ignore_ascii_case("RESOURCE") {
                    return Err(ParseError(format!(
                        "{}: unknown block: START {kind}",
                        line.at()
                    )));
                }
                let file = args.get(1).ok_or_else(|| {
                    ParseError(format!("{}: START RESOURCE without a file", line.at()))
                })?;
                self.block = Some(MmpResource {
                    file: file.clone(),
                    sourcepath: m.sourcepath.last().cloned(),
                    ..MmpResource::default()
                });
            }
            "TARGET" => m.target = args.join(" "),
            "TARGETTYPE" => m.target_type = args.join(" "),
            "UID" => {
                if args.len() != 2 && args.len() != 3 {
                    return Err(ParseError(format!(
                        "{}: UID requires two or three values",
                        line.at()
                    )));
                }
                m.uid.clear();
                for value in args {
                    m.uid.push(parse_uid_token(value)?);
                }
            }
            "TARGETPATH" => m.targetpath = Some(args.join(" ")),
            "SOURCE" => {
                for file in args {
                    m.source.push(file.clone());
                    m.source_sourcepath.push(m.sourcepath.last().cloned());
                }
            }
            "SOURCEPATH" => m.sourcepath.extend(args.iter().cloned()),
            "SYSTEMINCLUDE" => m.systeminclude.extend(args.iter().cloned()),
            "USERINCLUDE" => m.userinclude.extend(args.iter().cloned()),
            "LIBRARY" => m.library.extend(args.iter().cloned()),
            "STATICLIBRARY" => m.staticlibrary.extend(args.iter().cloned()),
            "CAPABILITY" => m.capability.extend(args.iter().cloned()),
            "EPOCSTACKSIZE" => m.epocstacksize = Some(args.join(" ")),
            "EPOCHEAPSIZE" => m.epocheapsize = Some(args.join(" ")),
            "EPOCALLOWDLLDATA" => m.epocallowdlldata = true,
            "DEFFILE" => m.deffile = Some(args.join(" ")),
            "NOSTRICTDEF" => m.nostrictdef = true,
            other => {
                return Err(ParseError(format!(
                    "{}: unknown directive: {other}",
                    line.at()
                )));
            }
        }
        Ok(())
    }
}

impl Mmp {
    /// Preprocessed `.mmp` text into the build model.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut parser = MmpParser::default();
        for line in ProjectLine::records(text) {
            parser.line(&line)?;
        }
        if parser.block.is_some() {
            return Err(ParseError("unclosed START RESOURCE".into()));
        }
        let mmp = parser.mmp;
        let kind = &mmp.target_type;
        if !kind.eq_ignore_ascii_case("EXE") && !kind.eq_ignore_ascii_case("DLL") {
            return Err(ParseError(format!(
                "TARGETTYPE must be EXE or DLL, got {kind}"
            )));
        }
        Ok(mmp)
    }
}

#[cfg(test)]
mod tests;
