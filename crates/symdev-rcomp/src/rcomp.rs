//! The `rcomp` job: recorded argv (`-u -o -h -s -i`) and the native compile.

use symdev_core::{Error, Result};

use crate::RscCompiled;
use crate::compiler::RssCompiler;
use crate::lexer::RssLexer;
use crate::parser::RssParser;

pub struct Rcomp {
    pub unicode: bool,
    pub rsc: String,
    pub header: Option<String>,
    pub source: String,
    pub input: String,
}

impl Rcomp {
    /// Compile the preprocessed `-s` source; write `-o` and, with `-h`, the header.
    /// Paths are host paths (the native tool takes no Wine `Z:` form).
    pub fn run(&self) -> Result<()> {
        if !self.unicode {
            return Err(Error::Other(
                "TODO: rcomp without -u (8-bit resources)".into(),
            ));
        }
        let src = std::fs::read(&self.source)
            .map_err(|e| Error::Other(format!("read {}: {e}", self.source)))?;
        let compiled = Self::compile(&src, &self.source)?;
        std::fs::write(&self.rsc, compiled.rsc_bytes()?)
            .map_err(|e| Error::Other(format!("write {}: {e}", self.rsc)))?;
        if let Some(h) = &self.header {
            std::fs::write(h, compiled.rsg_text())
                .map_err(|e| Error::Other(format!("write {h}: {e}")))?;
        }
        Ok(())
    }

    /// Preprocessed resource source → compiled resources.
    pub fn compile(src: &[u8], file: &str) -> Result<RscCompiled> {
        let tokens = RssLexer::new(src, file).tokens()?;
        let items = RssParser::new(tokens).items()?;
        RssCompiler::compile(&items)
    }

    pub fn from_args(args: &[String]) -> Result<Self> {
        let tokens = match args.first().map(String::as_str) {
            Some(s) if !s.starts_with('-') => &args[1..],
            _ => args,
        };
        let mut unicode = false;
        let mut rsc = None;
        let mut header = None;
        let mut source = None;
        let mut input = None;
        for tok in tokens {
            if tok == "-u" {
                unicode = true;
                continue;
            }
            // TODO: rcomp -v -p -l -force -{uid2,uid3} (recorded usage; unused on Wave 0)
            if tok == "-v" || tok == "-p" || tok == "-l" || tok == "-force" || tok.starts_with("-{")
            {
                return Err(Error::Other(format!("TODO: rcomp {tok}")));
            }
            if let Some(v) = tok.strip_prefix("-o") {
                rsc = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-h") {
                header = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-s") {
                source = Some(v.to_string());
            } else if let Some(v) = tok.strip_prefix("-i") {
                input = Some(v.to_string());
            } else if tok.starts_with('-') {
                return Err(Error::Other(format!("unknown rcomp flag: {tok}")));
            } else {
                return Err(Error::Other(format!("unexpected rcomp arg: {tok}")));
            }
        }
        match (rsc, source, input) {
            (Some(rsc), Some(source), Some(input)) => Ok(Self {
                unicode,
                rsc,
                header,
                source,
                input,
            }),
            _ => Err(Error::Other(
                "Usage: rcomp [-u] -oRSCFile [-hHeaderFile] -sSourceFile -iBaseInputFileName"
                    .into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_args_match_experiment_9() {
        let r = Rcomp::from_args(&[
            "rcomp".into(),
            "-u".into(),
            "-odriveinfo_reg.rsc".into(),
            "-sdriveinfo_reg.rpp".into(),
            "-idriveinfo_reg.rss".into(),
        ])
        .unwrap();
        assert!(r.unicode);
        assert_eq!(r.rsc, "driveinfo_reg.rsc");
        assert_eq!(r.header, None);
        assert_eq!(r.source, "driveinfo_reg.rpp");
        assert_eq!(r.input, "driveinfo_reg.rss");
    }

    #[test]
    fn from_args_with_header_match_experiment_41() {
        let r = Rcomp::from_args(&[
            "rcomp".into(),
            "-u".into(),
            "-odriveinfo_reg.rsc".into(),
            "-hdriveinfo_reg.rsg".into(),
            "-sdriveinfo_reg.rpp".into(),
            "-idriveinfo_reg.rss".into(),
        ])
        .unwrap();
        assert_eq!(r.header.as_deref(), Some("driveinfo_reg.rsg"));
    }
}
