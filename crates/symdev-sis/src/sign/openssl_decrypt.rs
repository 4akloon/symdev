//! Decrypting a traditional OpenSSL `ENCRYPTED DSA PRIVATE KEY` PEM (3DES-CBC, EVP_BytesToKey).
use symdev_core::{Error, Result};

use super::base64::base64_decode;

pub(super) fn decrypt_openssl_dsa_pem(text: &str, password: &str) -> Result<Vec<u8>> {
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
    use des::cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7};
    type TdesCbc = cbc::Decryptor<des::TdesEde3>;
    TdesCbc::new_from_slices(&key, &iv)
        .map_err(|_| Error::Other("invalid 3DES key/iv".into()))?
        .decrypt_padded_vec::<Pkcs7>(&ciphertext)
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
