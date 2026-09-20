//! `START BITMAP … END` (mmp-frontend-spec.md §7).
use crate::bld::ParseError;
use crate::model::{MmpBitmap, MmpBitmapSource};
use crate::project::ProjectLine;

/// A `START BITMAP` block while it is being read.
#[derive(Default)]
pub struct MmpBitmapBlock {
    pub bitmap: MmpBitmap,
    /// `SOURCEPATH` inside the block; it does not outlive the block (§7.2).
    sourcepath: Option<String>,
}

impl MmpBitmapBlock {
    pub fn open(line: &ProjectLine) -> Result<Self, ParseError> {
        let name = line.args().get(1).ok_or_else(|| {
            ParseError(format!("{}: START BITMAP without a file name", line.at()))
        })?;
        Ok(Self {
            bitmap: MmpBitmap {
                target: name.replace('/', "\\"),
                ..MmpBitmap::default()
            },
            sourcepath: None,
        })
    }

    /// One line inside the block; `Ok(true)` when `END` closed it.
    pub fn line(&mut self, line: &ProjectLine) -> Result<bool, ParseError> {
        let args = line.args();
        match line.directive().as_str() {
            "END" => return Ok(true),
            "HEADER" => self.bitmap.header = true,
            "TARGETPATH" => self.bitmap.targetpath = args.first().cloned(),
            "SOURCEPATH" => self.sourcepath = args.first().cloned(),
            "SOURCE" => {
                let [depths, files @ ..] = args else {
                    return Err(ParseError(format!(
                        "{}: SOURCE needs a depth list and at least one .bmp",
                        line.at()
                    )));
                };
                if files.is_empty() {
                    return Err(ParseError(format!(
                        "{}: SOURCE {depths} names no .bmp file",
                        line.at()
                    )));
                }
                let depths = Self::depths(depths, line)?;
                // §7.3: the depth list is cycled over the file list.
                for (i, file) in files.iter().enumerate() {
                    self.bitmap.sources.push(MmpBitmapSource {
                        file: file.to_ascii_lowercase().replace('/', "\\"),
                        sourcepath: self.sourcepath.clone(),
                        depth: depths[i % depths.len()].clone(),
                    });
                }
            }
            other => {
                return Err(ParseError(format!(
                    "{}: TODO: START BITMAP directive {other} (not observed)",
                    line.at()
                )));
            }
        }
        Ok(false)
    }

    /// A comma-separated depth list: each token an optional `c` and one or two decimal
    /// digits, lower-cased. Anything else is fatal (§7.3).
    fn depths(list: &str, line: &ProjectLine) -> Result<Vec<String>, ParseError> {
        let mut out = Vec::new();
        for token in list.split(',') {
            let token = token.to_ascii_lowercase();
            let digits = token.strip_prefix('c').unwrap_or(&token);
            let shaped =
                (1..=2).contains(&digits.len()) && digits.chars().all(|c| c.is_ascii_digit());
            if !shaped {
                return Err(ParseError(format!(
                    "{}: not a colour depth: {token}",
                    line.at()
                )));
            }
            out.push(token);
        }
        if out.is_empty() {
            return Err(ParseError(format!("{}: empty depth list", line.at())));
        }
        Ok(out)
    }
}
