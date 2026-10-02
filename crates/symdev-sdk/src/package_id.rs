use std::path::PathBuf;

use crate::{Result, SdkError};

/// A package's Android-style path, `kind;…;version`, with the full version always in it
/// (`gcce;12.1.0`, `sdk;s60-3rd-fp2;1.1`). A published id never changes; a rebuild is a
/// new version. Every segment is a safe directory name, so the id maps to an install path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(String);

impl PackageId {
    /// Splits `s` on `;` and checks every segment: non-empty, not starting with `.`, and
    /// free of `/`, `\`, NUL, whitespace and control characters.
    pub fn parse(s: &str) -> Result<Self> {
        let invalid = |reason| SdkError::InvalidId {
            id: s.to_string(),
            reason,
        };
        if s.is_empty() {
            return Err(invalid("is empty"));
        }
        if !s.contains(';') {
            return Err(invalid("needs a kind and a version"));
        }
        for segment in s.split(';') {
            if let Some(reason) = Self::segment_problem(segment) {
                return Err(invalid(reason));
            }
        }
        Ok(PackageId(s.to_string()))
    }

    /// An id written in the code (`Pins`); `Pins`'s tests check that each one parses.
    pub(crate) fn pinned(id: &'static str) -> Self {
        PackageId(id.to_string())
    }

    fn segment_problem(segment: &str) -> Option<&'static str> {
        if segment.is_empty() {
            Some("has an empty segment")
        } else if segment == "." || segment == ".." {
            Some("has a `.` or `..` segment")
        } else if segment.starts_with('.') {
            // `.staging` and `.lock` belong to the home, and `list` skips dot directories.
            Some("has a segment starting with `.`")
        } else if segment.contains(['/', '\\', '\0']) {
            Some("has a `/`, `\\` or NUL in a segment")
        } else if segment.chars().any(|c| c.is_whitespace() || c.is_control()) {
            Some("has whitespace or a control character")
        } else {
            None
        }
    }

    /// The first segment: `gcce`, `sdk`, …
    pub fn kind(&self) -> &str {
        self.segments().next().unwrap_or_default()
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split(';')
    }

    /// Where the package lives under `SYMDEV_HOME`: `gcce;12.1.0` → `gcce/12.1.0`.
    pub fn relative_path(&self) -> PathBuf {
        self.segments().collect()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The id as one shell word (`'gcce;12.1.0'`), for the commands that messages
    /// suggest: `;` separates shell commands, so the bare id would split the command.
    pub fn shell_word(&self) -> String {
        format!("'{}'", self.0.replace('\'', r"'\''"))
    }
}

impl std::fmt::Display for PackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for PackageId {
    type Err = SdkError;

    fn from_str(s: &str) -> Result<Self> {
        PackageId::parse(s)
    }
}

impl serde::Serialize for PackageId {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for PackageId {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        PackageId::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests;
