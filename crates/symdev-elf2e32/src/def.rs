//! Frozen export list (`.def`, `--definput`), as elf2e32_next reads it (experiment 54).

use symdev_core::{Error, Result};

/// One `name @ ordinal NONAME [DATA size] [ABSENT] [; comment]` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32DefEntry {
    pub name: String,
    pub ordinal: u32,
    /// `DATA <size>`: exported as data (`STT_OBJECT` in the `.dso`).
    pub data_size: Option<u32>,
    /// `ABSENT`: the ordinal is kept but the symbol is gone.
    pub absent: bool,
    /// Text from the first `;` to the end of the line, trailing blanks trimmed
    /// (elf2e32_next echoes it back after ` ; `).
    pub comment: Option<String>,
}

/// A frozen `.def`: entries in ordinal order, ordinals 1, 2, 3, … without gaps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct E32DefFile {
    pub entries: Vec<E32DefEntry>,
}

impl E32DefFile {
    pub fn parse(text: &str) -> Result<Self> {
        let mut entries = Vec::new();
        for (index, raw) in text.lines().enumerate() {
            let line_no = index + 1;
            let (body, comment) = match raw.find(';') {
                Some(at) => (&raw[..at], Some(raw[at..].trim_end().to_string())),
                None => (raw, None),
            };
            let body = body.trim();
            if body.is_empty() || body.eq_ignore_ascii_case("EXPORTS") {
                continue;
            }
            let entry = Self::entry(body, comment)
                .map_err(|e| Error::Other(format!(".def line {line_no}: {e}")))?;
            let expected = entries.len() as u32 + 1;
            if entry.ordinal != expected {
                return Err(Error::Other(format!(
                    ".def line {line_no}: ordinal {} of {} is not in sequence (expected {expected})",
                    entry.ordinal, entry.name
                )));
            }
            entries.push(entry);
        }
        Ok(Self { entries })
    }

    /// Marker `--defoutput` writes before exports that the frozen `.def` lacks.
    pub const NEW_MARKER: &str = "; NEW:";

    /// Entry lines under `; NEW:` in a `--defoutput` text.
    pub fn new_lines(generated: &str) -> Vec<&str> {
        generated
            .lines()
            .skip_while(|l| l.trim() != Self::NEW_MARKER)
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .collect()
    }

    /// Symbol names under `; NEW:` in a `--defoutput` text.
    pub fn new_names(generated: &str) -> Vec<&str> {
        Self::new_lines(generated)
            .into_iter()
            .filter_map(|l| l.split_whitespace().next())
            .collect()
    }

    /// Freeze: keep the existing `.def` text as it is and append the new entries from
    /// `--defoutput` (`None` when nothing is new). symdev's own rule, not a golden: SDK
    /// `efreeze.pl` does not run on this host (experiment 54).
    pub fn freeze(existing: Option<&str>, generated: &str) -> Result<Option<String>> {
        let new = Self::new_lines(generated);
        if new.is_empty() {
            return Ok(None);
        }
        let (mut out, nl) = match existing {
            Some(text) => {
                let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
                (text.trim_end().to_string(), nl)
            }
            None => ("EXPORTS".to_string(), "\n"),
        };
        for line in new {
            out.push_str(nl);
            out.push_str(line);
        }
        out.push_str(nl);
        out.push_str(nl);
        Self::parse(&out)?;
        Ok(Some(out))
    }

    fn entry(body: &str, comment: Option<String>) -> std::result::Result<E32DefEntry, String> {
        let mut words = body.split_whitespace();
        let name = words.next().ok_or("empty entry")?.trim_matches('"');
        if words.next() != Some("@") {
            return Err(format!("expected `{name} @ <ordinal>`"));
        }
        let ordinal = words
            .next()
            .and_then(|w| w.parse::<u32>().ok())
            .ok_or_else(|| format!("bad ordinal for {name}"))?;
        let mut entry = E32DefEntry {
            name: name.to_string(),
            ordinal,
            data_size: None,
            absent: false,
            comment,
        };
        let mut noname = false;
        while let Some(word) = words.next() {
            match word {
                "NONAME" => noname = true,
                "ABSENT" => entry.absent = true,
                "DATA" => {
                    let size = words
                        .next()
                        .and_then(|w| w.parse::<u32>().ok())
                        .ok_or_else(|| format!("DATA without a size for {name}"))?;
                    entry.data_size = Some(size);
                }
                other => return Err(format!("TODO: .def keyword {other} (not observed)")),
            }
        }
        if !noname {
            return Err(format!("TODO: export {name} without NONAME (not observed)"));
        }
        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sdk_style_def_with_comments_data_and_absent() {
        let def = E32DefFile::parse(
            "EXPORTS\r\n\t_ZN6CShapeC1Ei @ 1 NONAME\r\n; NEW:\n\t_ZTI6CShape @ 2 NONAME ; #<TI>#\n\
             \t_ZTV6CShape @ 3 NONAME DATA 24 ;  z  \n\t_Z8MathGonei @ 4 NONAME ABSENT\n\n",
        )
        .unwrap();
        assert_eq!(def.entries.len(), 4);
        assert_eq!(def.entries[1].comment.as_deref(), Some("; #<TI>#"));
        assert_eq!(def.entries[2].data_size, Some(24));
        assert_eq!(def.entries[2].comment.as_deref(), Some(";  z"));
        assert!(def.entries[3].absent);
        assert_eq!(def.entries[3].ordinal, 4);
    }

    #[test]
    fn freeze_appends_new_entries_and_keeps_the_frozen_text() {
        let generated =
            "EXPORTS\n\ta @ 1 NONAME\n; NEW:\n\tb @ 2 NONAME\n\tc @ 3 NONAME DATA 4\n\n";
        let existing = "EXPORTS\r\n\ta @ 1 NONAME ; keep me\r\n\r\n";
        assert_eq!(E32DefFile::new_names(generated), ["b", "c"]);
        assert_eq!(
            E32DefFile::freeze(Some(existing), generated)
                .unwrap()
                .unwrap(),
            "EXPORTS\r\n\ta @ 1 NONAME ; keep me\r\n\tb @ 2 NONAME\r\n\tc @ 3 NONAME DATA 4\r\n\r\n"
        );
        let first = "EXPORTS\n; NEW:\n\ta @ 1 NONAME\n\n";
        assert_eq!(
            E32DefFile::freeze(None, first).unwrap().unwrap(),
            "EXPORTS\n\ta @ 1 NONAME\n\n"
        );
        assert_eq!(
            E32DefFile::freeze(Some(existing), "EXPORTS\n\ta @ 1 NONAME\n\n").unwrap(),
            None
        );
    }

    #[test]
    fn rejects_ordinals_out_of_sequence_like_elf2e32_next() {
        // Experiment 54 (h, i, j): gaps and reordering are errors.
        for text in [
            "EXPORTS\n\ta @ 1 NONAME\n\tb @ 3 NONAME\n",
            "EXPORTS\n\ta @ 2 NONAME\n\tb @ 1 NONAME\n",
        ] {
            let err = E32DefFile::parse(text).unwrap_err().to_string();
            assert!(err.contains("not in sequence"), "{err}");
        }
    }
}
