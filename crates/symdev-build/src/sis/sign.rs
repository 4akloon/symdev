use super::array::SisArray;
use super::controller::SisController;
use super::field::SisEncode;
use super::signature::{SisAlgorithm38, SisBlob37, SisChain22, SisSignature36, SisSignatures39};
use super::string::SisString;
use der::{Decode, Sequence};
use dsa::signature::{DigestSigner, DigestVerifier, SignatureEncoding};
use dsa::{Signature, SigningKey, VerifyingKey};
use sha1::{Digest, Sha1};
use symdev_core::{Error, Result};

const DSA_WITH_SHA1: &str = "1.2.840.10040.4.3";

impl SisBlob37 {
    pub fn verify_dsa_sha1(&self, signed_bytes: &[u8], cert_der: &[u8]) -> Result<()> {
        let sig = parse_signature(&self.data)?;
        let vk = verifying_key_from_cert(cert_der)?;
        vk.verify_digest(Sha1::new_with_prefix(signed_bytes), &sig)
            .map_err(|_| Error::Other("DSA-SHA1 signature verify failed".into()))
    }
}

impl SisController {
    pub fn signatures_from_key_and_cert(
        &self,
        key_pem: &[u8],
        cert_pem_or_der: &[u8],
        password: &str,
    ) -> Result<SisSignatures39> {
        let cert_der = cert_to_der(cert_pem_or_der)?;
        let key = signing_key_from_pem(key_pem, password)?;
        let signed = self.signed_bytes();
        let sig: Signature = key
            .try_sign_digest(Sha1::new_with_prefix(&signed))
            .map_err(|_| Error::Other("DSA-SHA1 sign failed".into()))?;
        let mut blob = sig.to_bytes().to_vec();
        let pad = (4 - (blob.len() % 4)) % 4;
        blob.resize(blob.len() + pad, 0);
        let value = SisBlob37::new(blob);
        value.verify_dsa_sha1(&signed, &cert_der)?;
        Ok(SisSignatures39::new(
            SisArray::new(vec![
                SisSignature36::new(SisAlgorithm38::new(SisString::new(DSA_WITH_SHA1)), value)
                    .field(),
            ]),
            SisChain22::new(SisBlob37::new(cert_der)),
        ))
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

fn cert_to_der(pem_or_der: &[u8]) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(pem_or_der).unwrap_or("");
    if text.contains("BEGIN CERTIFICATE") {
        pem_body(pem_or_der, "CERTIFICATE")
    } else {
        Ok(pem_or_der.to_vec())
    }
}

fn signing_key_from_pem(pem: &[u8], password: &str) -> Result<SigningKey> {
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

fn decrypt_openssl_dsa_pem(text: &str, password: &str) -> Result<Vec<u8>> {
    let dek = text
        .lines()
        .find_map(|l| l.strip_prefix("DEK-Info: "))
        .ok_or_else(|| Error::Other("encrypted key missing DEK-Info".into()))?;
    let (cipher, iv_hex) = dek
        .split_once(',')
        .ok_or_else(|| Error::Other("encrypted key DEK-Info is malformed".into()))?;
    if cipher.trim() != "DES-EDE3-CBC" {
        return Err(Error::Other(format!(
            "unsupported private key cipher: {}",
            cipher.trim()
        )));
    }
    let iv = parse_iv_hex(iv_hex.trim())?;
    let begin = "-----BEGIN DSA PRIVATE KEY-----";
    let end = "-----END DSA PRIVATE KEY-----";
    let start = text
        .find(begin)
        .ok_or_else(|| Error::Other("missing DSA PRIVATE KEY".into()))?;
    let rest = &text[start + begin.len()..];
    let stop = rest
        .find(end)
        .ok_or_else(|| Error::Other("missing DSA PRIVATE KEY end".into()))?;
    let b64: String = rest[..stop]
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with("Proc-Type:") && !l.starts_with("DEK-Info:"))
        .flat_map(|l| l.chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    let ciphertext = base64_decode(&b64)?;
    let key = evp_bytes_to_key_md5(password.as_bytes(), &iv, 24);
    use des::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
    type TdesCbc = cbc::Decryptor<des::TdesEde3>;
    TdesCbc::new_from_slices(&key, &iv)
        .map_err(|_| Error::Other("invalid 3DES key/iv".into()))?
        .decrypt_padded_vec_mut::<Pkcs7>(&ciphertext)
        .map_err(|_| Error::Other("private key decrypt failed (check password)".into()))
}

fn parse_iv_hex(s: &str) -> Result<[u8; 8]> {
    if s.len() != 16 {
        return Err(Error::Other("DEK-Info IV must be 8 bytes hex".into()));
    }
    let mut out = [0u8; 8];
    for i in 0..8 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error::Other("DEK-Info IV is not hex".into()))?;
    }
    Ok(out)
}

fn evp_bytes_to_key_md5(password: &[u8], salt: &[u8], key_len: usize) -> Vec<u8> {
    use md5::{Digest, Md5};
    let mut out = Vec::new();
    let mut prev: Vec<u8> = Vec::new();
    while out.len() < key_len {
        let mut h = Md5::new();
        h.update(&prev);
        h.update(password);
        h.update(salt);
        prev = h.finalize().to_vec();
        out.extend_from_slice(&prev);
    }
    out.truncate(key_len);
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            break;
        }
        let a = val(bytes[i]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        let b = if i + 1 < bytes.len() && bytes[i + 1] != b'=' {
            val(bytes[i + 1]).ok_or_else(|| Error::Other("invalid base64".into()))?
        } else {
            break;
        };
        out.push((a << 2) | (b >> 4));
        if i + 2 >= bytes.len() || bytes[i + 2] == b'=' {
            break;
        }
        let c = val(bytes[i + 2]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        out.push((b << 4) | (c >> 2));
        if i + 3 >= bytes.len() || bytes[i + 3] == b'=' {
            break;
        }
        let d = val(bytes[i + 3]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        out.push((c << 6) | d);
        i += 4;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sis::{
        SisArray, SisDate, SisDateTime, SisFile, SisFiles, SisHash, SisInfo, SisLanguage,
        SisLanguages, SisPkgUid, SisProduct, SisProductVersion, SisProducts, SisString, SisTime,
        SisU32, SisVersion, SisWord41, SisWords, SisWords16, SisWords19,
    };

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_type39_golden() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_type39.hex"))
    }

    fn hello_sig_blob() -> [u8; 48] {
        [
            0x30, 0x2c, 0x02, 0x14, 0x4d, 0xe0, 0xcf, 0xec, 0x52, 0x8a, 0x05, 0x95, 0x13, 0x7d,
            0xfc, 0x0c, 0x66, 0x34, 0xe4, 0x00, 0x75, 0x28, 0xae, 0xa0, 0x02, 0x14, 0x50, 0x20,
            0x97, 0x21, 0xc3, 0x8a, 0xb4, 0xdd, 0xb9, 0xc0, 0x1d, 0x71, 0x53, 0xd3, 0x3d, 0xe7,
            0x10, 0x62, 0xa8, 0xc0, 0x00, 0x00,
        ]
    }

    fn hello_der() -> Vec<u8> {
        hello_type39_golden()[148..148 + 1171].to_vec()
    }

    fn hello_controller() -> SisController {
        SisController::new(
            SisInfo::new(
                SisPkgUid::new(0xe79e_4cf9),
                SisString::new("Vendor"),
                SisArray::new(vec![SisString::new("hello").field()]),
                SisArray::new(vec![SisString::new("Vendor-EN").field()]),
                SisVersion::new(1, 0, 24),
                SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
            ),
            SisWords16::new(SisWords::new(vec![0x21])),
            SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
            SisProducts::new(SisArray::new(vec![
                SisProduct::new(
                    SisPkgUid::new(0x1027_52ae),
                    SisProductVersion::new(SisVersion::new(0, 0, 0)),
                    SisArray::new(vec![SisString::new("S60ProductID").field()]),
                )
                .field(),
            ])),
            SisWords19::new(SisWords::new(vec![0x14])),
            SisFiles::new(
                SisArray::new(vec![
                    SisFile::new(
                        SisString::new("!:\\sys\\bin\\hello.exe"),
                        SisString::new(""),
                        SisWord41::new(0x000b_e000),
                        SisHash::new(
                            [1, 0x25, 0x14],
                            [
                                0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b,
                                0x2a, 0x77, 0xe5, 0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
                            ],
                        ),
                        SisString::new(""),
                        [3588, 0, 3588, 0, 0],
                    )
                    .field(),
                ]),
                SisWords::new(vec![0x0d]),
                SisWords::new(vec![0x1a]),
            ),
            SisU32::new(0),
        )
    }

    #[test]
    fn hello_frozen_dsa_signature_verifies_signed_bytes() {
        let signed = hello_controller().signed_bytes();
        assert_eq!(signed.len(), 528);
        SisBlob37::new(hello_sig_blob().to_vec())
            .verify_dsa_sha1(&signed, &hello_der())
            .unwrap();
    }

    #[test]
    fn hello_frozen_dsa_signature_rejects_wrong_bytes() {
        let mut signed = hello_controller().signed_bytes();
        signed[0] ^= 1;
        assert!(
            SisBlob37::new(hello_sig_blob().to_vec())
                .verify_dsa_sha1(&signed, &hello_der())
                .is_err()
        );
    }

    fn type37_blob(sigs: &SisSignatures39) -> Vec<u8> {
        let field = sigs.field().bytes();
        let n37 = u32::from_le_bytes(field[76 + 4..76 + 8].try_into().unwrap()) as usize;
        field[76 + 8..76 + 8 + n37].to_vec()
    }

    fn test_cert_der() -> Vec<u8> {
        parse_hex(include_str!("testdata/test_dsa_cert.hex"))
    }

    #[test]
    fn live_sign_traditional_pem_verifies() {
        let controller = hello_controller();
        let key = parse_hex(include_str!("testdata/test_dsa_key.hex"));
        let cert = test_cert_der();
        let signed = controller
            .signatures_from_key_and_cert(&key, &cert, "")
            .unwrap();
        assert_eq!(signed.chain.cert.data, cert);
        SisBlob37::new(type37_blob(&signed))
            .verify_dsa_sha1(&controller.signed_bytes(), &cert)
            .unwrap();
    }

    #[test]
    fn live_sign_encrypted_traditional_pem_verifies() {
        let controller = hello_controller();
        let key = parse_hex(include_str!("testdata/test_dsa_key_3des.hex"));
        let cert = test_cert_der();
        let signed = controller
            .signatures_from_key_and_cert(&key, &cert, "test")
            .unwrap();
        SisBlob37::new(type37_blob(&signed))
            .verify_dsa_sha1(&controller.signed_bytes(), &cert)
            .unwrap();
    }

    #[test]
    fn live_sign_pkcs8_pem_verifies() {
        let controller = hello_controller();
        let key = parse_hex(include_str!("testdata/test_dsa_key_pkcs8.hex"));
        let cert = test_cert_der();
        let signed = controller
            .signatures_from_key_and_cert(&key, &cert, "")
            .unwrap();
        SisBlob37::new(type37_blob(&signed))
            .verify_dsa_sha1(&controller.signed_bytes(), &cert)
            .unwrap();
    }
}
