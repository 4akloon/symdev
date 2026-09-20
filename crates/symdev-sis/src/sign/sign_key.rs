//! `SisController::signatures_from_key_and_cert`: the DSA-SHA1 signing entry point.
use dsa::Signature;
use dsa::signature::{DigestSigner, SignatureEncoding};
use sha1::{Digest, Sha1};
use symdev_core::{Error, Result};

use super::private_key::{cert_to_der, signing_key_from_pem};
use crate::{
    SisAlgorithm38, SisArray, SisBlob37, SisChain22, SisController, SisEncode, SisSignature36,
    SisSignatures39, SisString,
};

const DSA_WITH_SHA1: &str = "1.2.840.10040.4.3";

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
