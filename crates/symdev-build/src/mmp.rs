//! `Mmp::parse`: the `.mmp` grammar
//! ([mmp-frontend-spec.md](../../../docs/research/mmp-frontend-spec.md) §5), over text
//! the preprocessor has already been through (`ProjectCpp`).
mod bitmap;
mod capability;
mod directives;

use crate::bld::ParseError;
use crate::model::{Mmp, MmpOption, MmpResource};
use crate::project::ProjectLine;

use bitmap::MmpBitmapBlock;
use directives::{IGNORED, number, rejected};

/// The `.mmp` fields as they accumulate, one line at a time.
#[derive(Default)]
struct MmpParser {
    mmp: Mmp,
    block: Option<MmpResource>,
    bitmap: Option<MmpBitmapBlock>,
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
            "TARGET" => block.target = args.first().cloned(),
            "HEADER" => block.header = true,
            "HEADERONLY" => block.headeronly = true,
            "TARGETPATH" => block.targetpath = args.first().cloned(),
            "LANG" => block.lang.extend(args.iter().cloned()),
            "UID" => {
                return Err(ParseError(format!(
                    "{}: TODO: UID inside START RESOURCE (symdev's rcomp has no \
                     -uid2/-uid3 yet, so the resource's UIDs would be lost)",
                    line.at()
                )));
            }
            other => {
                return Err(ParseError(format!(
                    "{}: TODO: START RESOURCE directive {other} (not observed)",
                    line.at()
                )));
            }
        }
        Ok(())
    }

    /// `MACRO`, `OPTION`, `SECUREID`, `VENDORID` and the directives that only reach the
    /// model (§5.3, §5.5).
    fn extra_line(&mut self, name: &str, line: &ProjectLine) -> Result<bool, ParseError> {
        let args = line.args();
        let m = &mut self.mmp;
        match name {
            "MACRO" => m.macros.extend(args.iter().cloned()),
            "LANG" => m.lang.extend(args.iter().cloned()),
            "OPTION" => {
                let [compiler, text @ ..] = args else {
                    return Err(ParseError(format!(
                        "{}: OPTION needs a compiler and at least one flag",
                        line.at()
                    )));
                };
                let compiler = compiler.to_ascii_uppercase();
                let text = text.join(" ");
                match m.options.iter_mut().find(|o| o.compiler == compiler) {
                    Some(existing) => {
                        existing.text.push(' ');
                        existing.text.push_str(&text);
                    }
                    None => m.options.push(MmpOption { compiler, text }),
                }
            }
            "SECUREID" => m.secureid = Some(Self::one_number(line)?),
            "VENDORID" => {
                let value = Self::one_number(line)?;
                if value != 0 {
                    return Err(ParseError(format!(
                        "{}: TODO: VENDORID 0x{value:08x} (symdev's post-linker has no --vid, \
                         so the image would silently carry vendor id 0)",
                        line.at()
                    )));
                }
                m.vendorid = Some(0);
            }
            _ if IGNORED.contains(&name) => m.warnings.push(format!(
                "{}: {name} is parsed but nothing on the GCCE path reads it",
                line.at()
            )),
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn one_number(line: &ProjectLine) -> Result<u32, ParseError> {
        let value = line.args().first().ok_or_else(|| {
            ParseError(format!("{}: {} needs a value", line.at(), line.tokens[0]))
        })?;
        number(value).map_err(|e| ParseError(format!("{}: {e}", line.at())))
    }

    fn line(&mut self, line: &ProjectLine) -> Result<(), ParseError> {
        if self.block.is_some() {
            return self.resource_line(line);
        }
        if let Some(block) = self.bitmap.as_mut() {
            if block.line(line)?
                && let Some(block) = self.bitmap.take()
            {
                self.mmp.bitmap.push(block.bitmap);
            }
            return Ok(());
        }
        let name = line.directive();
        let args = line.args();
        let m = &mut self.mmp;
        match name.as_str() {
            "START" => return self.start(line),
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
                    m.uid.push(
                        number(value).map_err(|e| ParseError(format!("{}: {e}", line.at())))?,
                    );
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
            "EPOCALLOWDLLDATA" => m.epocallowdlldata = true,
            "DEFFILE" => m.deffile = Some(args.join(" ")),
            "NOSTRICTDEF" => m.nostrictdef = true,
            other => {
                if let Some(why) = rejected(other) {
                    return Err(ParseError(format!("{}: {other}: {why}", line.at())));
                }
                if !self.extra_line(other, line)? {
                    return Err(ParseError(format!(
                        "{}: unknown directive: {other}",
                        line.at()
                    )));
                }
            }
        }
        Ok(())
    }

    /// `START <kind>`: a resource block, or a block this platform skips (§5.2).
    fn start(&mut self, line: &ProjectLine) -> Result<(), ParseError> {
        let args = line.args();
        let kind = args
            .first()
            .cloned()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if kind == "BITMAP" {
            self.bitmap = Some(MmpBitmapBlock::open(line)?);
            return Ok(());
        }
        if kind != "RESOURCE" {
            return Err(ParseError(format!(
                "{}: TODO: START {kind} (not observed)",
                line.at()
            )));
        }
        let file = args
            .get(1)
            .ok_or_else(|| ParseError(format!("{}: START RESOURCE without a file", line.at())))?;
        self.block = Some(MmpResource {
            file: file.clone(),
            sourcepath: self.mmp.sourcepath.last().cloned(),
            ..MmpResource::default()
        });
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
        if parser.bitmap.is_some() {
            return Err(ParseError("unclosed START BITMAP".into()));
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

    /// The `OPTION` text for one compiler key, upper-cased as §8.5 matches it.
    pub fn option(&self, compiler: &str) -> Option<&str> {
        self.options
            .iter()
            .find(|o| o.compiler.eq_ignore_ascii_case(compiler))
            .map(|o| o.text.as_str())
    }
}

pub use capability::MmpCapabilities;

#[cfg(test)]
mod tests;
