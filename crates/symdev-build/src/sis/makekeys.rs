//! Native stand-in for Wine `makekeys -cert`.
//!
//! Emits a self-signed DSA-SHA1 cert (OID `1.2.840.10040.4.3`) plus PKCS#8
//! `BEGIN PRIVATE KEY` PEM that `encode_signed_sisx` already reads.
//! Wine `makekeys` writes encrypted traditional `BEGIN DSA PRIVATE KEY`
//! (`Proc-Type: 4,ENCRYPTED` / `DEK-Info: DES-EDE3-CBC`); that format is not
//! re-emitted. Native sign can still *load* those Wine keys when a password is
//! supplied.
//!
//! ponytail: `DSA_1024_160` (FIPS DSA-SHA1, same q=160 as frozen hello.cer).
//! makekeys `-len 2048` produced 2048/160; `dsa` 0.6 has no that size, and
//! `DSA_2048_256` took ~192s per keygen in debug tests.

use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use der::asn1::{BitString, UtcTime};
use der::{Decode, Encode};
use dsa::pkcs8::EncodePrivateKey;
use dsa::signature::DigestSigner;
use dsa::signature::SignatureEncoding;
use dsa::{Components, KeySize, SigningKey};
use sha1::{Digest, Sha1};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::{AlgorithmIdentifierOwned, EncodePublicKey, ObjectIdentifier};
use x509_cert::time::{Time, Validity};
use x509_cert::{Certificate, TbsCertificate, Version};

use symdev_core::{Error, Result};

const DSA_WITH_SHA1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10040.4.3");
const EXPDAYS: u64 = 3650;
const SUBJECT: &str =
    "CN=Joe Bloggs,OU=Development,O=Acme Ltd,C=GB,emailAddress=noone@nowhere.com";

pub fn generate_self_signed_dsa(not_before: SystemTime) -> Result<(Vec<u8>, Vec<u8>)> {
    let not_after = not_before
        .checked_add(Duration::from_secs(EXPDAYS * 86_400))
        .ok_or_else(|| Error::Other("certificate expiry overflow".into()))?;
    let mut rng = rand::rngs::OsRng;
    #[allow(deprecated)]
    let components = Components::generate(&mut rng, KeySize::DSA_1024_160);
    let key = SigningKey::generate(&mut rng, components);
    let spki = x509_cert::spki::SubjectPublicKeyInfoOwned::from_der(
        key.verifying_key()
            .to_public_key_der()
            .map_err(|e| Error::Other(format!("DSA SPKI: {e}")))?
            .as_bytes(),
    )
    .map_err(|e| Error::Other(format!("DSA SPKI DER: {e}")))?;
    let subject = Name::from_str(SUBJECT).map_err(|e| Error::Other(format!("dname: {e}")))?;
    let alg = AlgorithmIdentifierOwned {
        oid: DSA_WITH_SHA1,
        parameters: None,
    };
    let tbs = TbsCertificate {
        version: Version::V3,
        serial_number: SerialNumber::from(1u32),
        signature: alg.clone(),
        issuer: subject.clone(),
        validity: Validity {
            not_before: utc(not_before)?,
            not_after: utc(not_after)?,
        },
        subject,
        subject_public_key_info: spki,
        issuer_unique_id: None,
        subject_unique_id: None,
        extensions: None,
    };
    let tbs_der = tbs
        .to_der()
        .map_err(|e| Error::Other(format!("TBS encode: {e}")))?;
    let sig: dsa::Signature = key
        .try_sign_digest(Sha1::new_with_prefix(&tbs_der))
        .map_err(|_| Error::Other("DSA-SHA1 certificate sign failed".into()))?;
    let cert = Certificate {
        tbs_certificate: tbs,
        signature_algorithm: alg,
        signature: BitString::from_bytes(&sig.to_bytes())
            .map_err(|e| Error::Other(format!("signature BIT STRING: {e}")))?,
    };
    let cert_der = cert
        .to_der()
        .map_err(|e| Error::Other(format!("certificate encode: {e}")))?;
    let key_der = key
        .to_pkcs8_der()
        .map_err(|e| Error::Other(format!("PKCS#8 encode: {e}")))?;
    Ok((
        pem_encode("CERTIFICATE", &cert_der),
        pem_encode("PRIVATE KEY", key_der.as_bytes()),
    ))
}

fn utc(t: SystemTime) -> Result<Time> {
    let secs = t
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::Other("certificate date before Unix epoch".into()))?;
    UtcTime::from_unix_duration(secs)
        .map(Time::UtcTime)
        .map_err(|e| Error::Other(format!("UTCTime: {e}")))
}

fn pem_encode(label: &str, der: &[u8]) -> Vec<u8> {
    let b64 = base64_encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap());
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out.into_bytes()
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        out.push(T[(a >> 2) as usize] as char);
        out.push(T[(((a & 3) << 4) | (b >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b & 15) << 2) | (c >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(c & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}
