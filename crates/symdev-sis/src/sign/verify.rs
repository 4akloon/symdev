//! `SisBlob37::verify_dsa_sha1`: verifying a DSA-SHA1 signature against a certificate.
use der::Decode;
use dsa::signature::DigestVerifier;
use dsa::{Signature, VerifyingKey};
use sha1::{Digest, Sha1};
use symdev_core::{Error, Result};

use crate::SisBlob37;

impl SisBlob37 {
    pub fn verify_dsa_sha1(&self, signed_bytes: &[u8], cert_der: &[u8]) -> Result<()> {
        let sig = parse_signature(&self.data)?;
        let vk = verifying_key_from_cert(cert_der)?;
        vk.verify_digest(Sha1::new_with_prefix(signed_bytes), &sig)
            .map_err(|_| Error::Other("DSA-SHA1 signature verify failed".into()))
    }
}

fn parse_signature(blob: &[u8]) -> Result<Signature> {
    let der = der_prefix(blob)?;
    Signature::try_from(der).map_err(|_| Error::Other("invalid DSA signature DER".into()))
}

fn der_prefix(blob: &[u8]) -> Result<&[u8]> {
    if blob.len() < 2 || blob[0] != 0x30 {
        return Err(Error::Other("invalid DSA signature DER".into()));
    }
    let len = blob[1] as usize;
    if len >= 0x80 || blob.len() < 2 + len {
        return Err(Error::Other("invalid DSA signature DER".into()));
    }
    Ok(&blob[..2 + len])
}

fn verifying_key_from_cert(cert_der: &[u8]) -> Result<VerifyingKey> {
    use der::Encode;
    use dsa::pkcs8::DecodePublicKey;
    let cert = x509_cert::Certificate::from_der(cert_der)
        .map_err(|e| Error::Other(format!("invalid certificate DER: {e}")))?;
    let spki = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| Error::Other(format!("certificate SPKI: {e}")))?;
    VerifyingKey::from_public_key_der(&spki)
        .map_err(|_| Error::Other("certificate is not a DSA public key".into()))
}
