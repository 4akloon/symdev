//! PEM/DER decoding of the certificate and DSA private key used to sign.
use der::{Decode, Sequence};
use dsa::{SigningKey, VerifyingKey};
use symdev_core::{Error, Result};

use super::base64::base64_decode;
use super::openssl_decrypt::decrypt_openssl_dsa_pem;

pub(super) fn cert_to_der(pem_or_der: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(pem_or_der).unwrap_or("");
    if text.contains("BEGIN CERTIFICATE") {
        pem_body(pem_or_der, "CERTIFICATE")
    } else {
        Ok(pem_or_der.to_vec())
    }
}

pub(super) fn signing_key_from_pem(pem: &[u8], password: &str) -> Result<SigningKey> {
    let text = std::str::from_utf8(pem)
        .map_err(|_| Error::Other("signing key is not UTF-8 PEM".into()))?;
    if text.contains("BEGIN PRIVATE KEY") {
        use dsa::pkcs8::DecodePrivateKey;
        let der = pem_body(pem, "PRIVATE KEY")?;
        return SigningKey::from_pkcs8_der(&der)
            .map_err(|_| Error::Other("invalid PKCS#8 DSA private key".into()));
    }
    if !text.contains("BEGIN DSA PRIVATE KEY") {
        return Err(Error::Other("unsupported private key PEM".into()));
    }
    let der = if text.contains("ENCRYPTED") {
        decrypt_openssl_dsa_pem(text, password)?
    } else {
        pem_body(pem, "DSA PRIVATE KEY")?
    };
    signing_key_from_traditional_der(&der)
}

fn signing_key_from_traditional_der(der: &[u8]) -> Result<SigningKey> {
    let decoded = TraditionalDsaKey::from_der(der)
        .map_err(|_| Error::Other("invalid traditional DSA private key".into()))?;
    let p = dsa::BigUint::from_bytes_be(decoded.p.as_bytes());
    let q = dsa::BigUint::from_bytes_be(decoded.q.as_bytes());
    let g = dsa::BigUint::from_bytes_be(decoded.g.as_bytes());
    let y = dsa::BigUint::from_bytes_be(decoded.y.as_bytes());
    let x = dsa::BigUint::from_bytes_be(decoded.x.as_bytes());
    let components = dsa::Components::from_components(p, q, g)
        .map_err(|_| Error::Other("invalid DSA parameters".into()))?;
    let vk = VerifyingKey::from_components(components, y)
        .map_err(|_| Error::Other("invalid DSA public component".into()))?;
    SigningKey::from_components(vk, x)
        .map_err(|_| Error::Other("invalid DSA private component".into()))
}

#[derive(Sequence)]
struct TraditionalDsaKey<'a> {
    _version: der::asn1::UintRef<'a>,
    p: der::asn1::UintRef<'a>,
    q: der::asn1::UintRef<'a>,
    g: der::asn1::UintRef<'a>,
    y: der::asn1::UintRef<'a>,
    x: der::asn1::UintRef<'a>,
}

fn pem_body(pem: &[u8], label: &str) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(pem).map_err(|_| Error::Other("PEM is not UTF-8".into()))?;
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let start = text
        .find(&begin)
        .ok_or_else(|| Error::Other(format!("missing {begin}")))?;
    let rest = &text[start + begin.len()..];
    let stop = rest
        .find(&end)
        .ok_or_else(|| Error::Other(format!("missing {end}")))?;
    let b64: String = rest[..stop]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    base64_decode(&b64)
}
