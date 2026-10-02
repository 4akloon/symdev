use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::Signature;

use crate::{IndexSigningKey, Result, SdkError, TrustedKeys};

/// How a signature line starts; `ed25519 <base64 of the 64-byte signature>` follows.
const PREFIX: &str = "# symdev-signature: ";
const ALGORITHM: &str = "ed25519 ";

/// An `index.toml` as a source serves it: an optional first line
/// `# symdev-signature: ed25519 <base64>` and the body, exactly the bytes after that
/// line's `\n`, which the signature covers. To TOML the line is a comment, so a reader that
/// knows nothing of signatures (symdev 0.1.0) still reads the index; and index and
/// signature are one object, so no upload can leave them disagreeing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedIndex {
    /// The first line, without its `\n`, when it starts with [`PREFIX`].
    line: Option<String>,
    body: String,
}

impl SignedIndex {
    /// Splits `text` into its signature line, if the first line is one, and the body.
    /// A malformed signature line is kept; [`SignedIndex::verify`] refuses it.
    pub fn split(text: &str) -> SignedIndex {
        if !text.starts_with(PREFIX) {
            return SignedIndex {
                line: None,
                body: text.to_string(),
            };
        }
        let (line, body) = text.split_once('\n').unwrap_or((text, ""));
        SignedIndex {
            line: Some(line.to_string()),
            body: body.to_string(),
        }
    }

    /// `body` with a signature line by `key`.
    pub fn sign(body: &str, key: &IndexSigningKey) -> SignedIndex {
        let signature = STANDARD.encode(key.sign(body.as_bytes()));
        SignedIndex {
            line: Some(format!("{PREFIX}{ALGORITHM}{signature}")),
            body: body.to_string(),
        }
    }

    /// Whether the first line is a signature line (well-formed or not).
    pub fn is_signed(&self) -> bool {
        self.line.is_some()
    }

    /// The index without its signature line, verified or not.
    pub fn body(&self) -> &str {
        &self.body
    }

    /// The text to serve: the signature line, if any, then the body.
    pub fn to_text(&self) -> String {
        match &self.line {
            Some(line) => format!("{line}\n{}", self.body),
            None => self.body.clone(),
        }
    }

    /// The body, if one of `keys` signed it. Otherwise an error naming `url`, the index's
    /// URL: the index is unsigned, its signature line is malformed, or the signature does
    /// not verify (the body changed after signing, or another key signed it).
    pub fn verify(&self, keys: &TrustedKeys, url: &str) -> Result<&str> {
        let refuse = |detail: String| SdkError::UntrustedIndex {
            url: url.to_string(),
            detail,
        };
        let Some(line) = &self.line else {
            return Err(refuse(format!(
                "the index is unsigned (its first line is not `{PREFIX}{ALGORITHM}…`), but \
                 this source accepts only an index signed by {}; it may have been tampered \
                 with, so it was not used",
                keys.describe()
            )));
        };
        let signature = Self::signature(line).map_err(|why| {
            refuse(format!(
                "the index's signature line is malformed ({why}); it may have been tampered \
                 with, so it was not used"
            ))
        })?;
        if keys.signed(self.body.as_bytes(), &signature) {
            Ok(&self.body)
        } else {
            Err(refuse(format!(
                "the index's signature does not verify with {}: the index was tampered with \
                 after it was signed, or another key signed it, so it was not used",
                keys.describe()
            )))
        }
    }

    fn signature(line: &str) -> std::result::Result<Signature, &'static str> {
        let encoded = line
            .strip_prefix(PREFIX)
            .and_then(|rest| rest.strip_prefix(ALGORITHM))
            .ok_or("only `ed25519` signatures are known")?;
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| "the signature is not base64")?;
        let bytes: [u8; 64] = bytes
            .try_into()
            .map_err(|_| "the signature is not 64 bytes")?;
        Ok(Signature::from_bytes(&bytes))
    }
}

#[cfg(test)]
mod tests;
