use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::{Result, SdkError};

/// symdev's own index-signing public keys (Ed25519), which the built-in source's index
/// must be signed with. A key rotation ships a symdev that lists both keys before the
/// indexes are re-signed with the new one. The spec (§11) records each key's fingerprint.
const BUILTIN: [[u8; 32]; 1] = [
    // C1yh60B72Qa4YE4rZOgoPJZmTYKbh/uzHjupoVwqfLU=, generated 2026-10-03; fingerprint
    // bdf5345cc3ca8c30661dbc53b2cbd16983d081bf26913d0c3ebe7480ca334d44.
    [
        0x0b, 0x5c, 0xa1, 0xeb, 0x40, 0x7b, 0xd9, 0x06, 0xb8, 0x60, 0x4e, 0x2b, 0x64, 0xe8, 0x28,
        0x3c, 0x96, 0x66, 0x4d, 0x82, 0x9b, 0x87, 0xfb, 0xb3, 0x1e, 0x3b, 0xa9, 0xa1, 0x5c, 0x2a,
        0x7c, 0xb5,
    ],
];

/// The Ed25519 public keys an index signature is checked against: symdev's own for the
/// built-in source, or what a source's `key` in `sources.toml` names. A signature by any
/// of them is accepted, so a key can be rotated.
#[derive(Clone, PartialEq, Eq)]
pub struct TrustedKeys {
    keys: Vec<VerifyingKey>,
}

impl TrustedKeys {
    /// symdev's own keys. Each is a constant that this module's test checks against the
    /// key the spec records, so a malformed one cannot ship.
    pub fn builtin() -> TrustedKeys {
        TrustedKeys {
            keys: BUILTIN
                .iter()
                .filter_map(|key| VerifyingKey::from_bytes(key).ok())
                .collect(),
        }
    }

    /// A source's `key` in `sources.toml`: `builtin` (symdev's own keys) or the base64 of
    /// one 32-byte Ed25519 public key. Surrounding whitespace is ignored.
    pub fn parse(text: &str) -> Result<TrustedKeys> {
        let text = text.trim();
        if text == "builtin" {
            return Ok(Self::builtin());
        }
        let bad = |why: String| {
            SdkError::Other(format!(
                "key `{text}` {why}; give \"builtin\" (symdev's own key) or the base64 of a \
                 32-byte Ed25519 public key"
            ))
        };
        let bytes = STANDARD
            .decode(text)
            .map_err(|_| bad("is not base64".into()))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|b: Vec<u8>| bad(format!("is {} bytes, not 32", b.len())))?;
        let key = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| bad("is not a point of the Ed25519 curve".into()))?;
        Ok(Self::from_key(key))
    }

    pub(crate) fn from_key(key: VerifyingKey) -> TrustedKeys {
        TrustedKeys { keys: vec![key] }
    }

    /// These keys and `other`'s, each listed once.
    pub fn and(mut self, other: TrustedKeys) -> TrustedKeys {
        for key in other.keys {
            if !self.keys.contains(&key) {
                self.keys.push(key);
            }
        }
        self
    }

    /// Each key's fingerprint: the SHA-256 of its 32 bytes, in lowercase hex.
    pub fn fingerprints(&self) -> Vec<String> {
        self.keys
            .iter()
            .map(|key| format!("{:x}", Sha256::digest(key.as_bytes())))
            .collect()
    }

    /// Whether one of the keys signed `body` (RFC 8032, strict: no malleable signature or
    /// weak key is accepted).
    pub(crate) fn signed(&self, body: &[u8], signature: &Signature) -> bool {
        self.keys
            .iter()
            .any(|key| key.verify_strict(body, signature).is_ok())
    }

    /// The keys as a message names them, by the first 16 digits of their fingerprints.
    pub(crate) fn describe(&self) -> String {
        let short: Vec<String> = self
            .fingerprints()
            .iter()
            .map(|f| f.chars().take(16).collect())
            .collect();
        match short.as_slice() {
            [] => "no key (none is trusted)".into(),
            [one] => format!("the key with fingerprint {one}…"),
            many => format!("any of the keys with fingerprints {}…", many.join("…, ")),
        }
    }
}

impl fmt::Debug for TrustedKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TrustedKeys")
            .field(&self.fingerprints())
            .finish()
    }
}

#[cfg(test)]
mod tests;
