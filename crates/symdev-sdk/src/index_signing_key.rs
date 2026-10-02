use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use zeroize::Zeroizing;

use crate::{Result, SdkError, TrustedKeys};

/// The key a publisher signs indexes with: an Ed25519 seed, which the publisher's
/// environment holds as base64 (`PUBLISH_SIGNING_KEY`). Dropping it wipes the seed, and
/// `Debug` shows only the public key's fingerprint.
pub struct IndexSigningKey {
    key: SigningKey,
}

impl IndexSigningKey {
    /// `text` is the base64 of the 32-byte seed; surrounding whitespace is ignored. The
    /// error never repeats `text`, which is a secret.
    pub fn from_base64(text: &str) -> Result<IndexSigningKey> {
        let bad = || {
            SdkError::Other(
                "the index signing key is not the base64 of a 32-byte Ed25519 seed (the last \
                 32 bytes of the key's PKCS#8 DER, as the toolchain spec's §15 shows)"
                    .into(),
            )
        };
        let bytes = Zeroizing::new(STANDARD.decode(text.trim()).map_err(|_| bad())?);
        let seed: Zeroizing<[u8; 32]> =
            Zeroizing::new(bytes.as_slice().try_into().map_err(|_| bad())?);
        Ok(IndexSigningKey {
            key: SigningKey::from_bytes(&seed),
        })
    }

    /// The public half, as the keys that verify what this key signs.
    pub fn trusted(&self) -> TrustedKeys {
        TrustedKeys::from_key(self.key.verifying_key())
    }

    /// The Ed25519 signature of `body`.
    pub(crate) fn sign(&self, body: &[u8]) -> [u8; 64] {
        self.key.sign(body).to_bytes()
    }
}

impl fmt::Debug for IndexSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IndexSigningKey")
            .field("public", &self.trusted())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
