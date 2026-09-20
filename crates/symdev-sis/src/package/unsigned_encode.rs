//! `SisUnsigned`: encoding a whole unsigned or signed SIS from a spec.
use symdev_core::Result;

use super::spec::SisUnsignedSpec;
use crate::{SisCompressed, SisController, SisData, SisEncode, SisUid, SisUnsigned};

impl SisUnsigned {
    pub fn encode(spec: &SisUnsignedSpec<'_>) -> Result<Vec<u8>> {
        let (controller, data) = spec.parts()?;
        Self::wrap(spec.uid3, &controller, data)
    }

    pub fn encode_signed(
        spec: &SisUnsignedSpec<'_>,
        key_pem: &[u8],
        cert_pem_or_der: &[u8],
        password: &str,
    ) -> Result<Vec<u8>> {
        let (controller, data) = spec.parts()?;
        let signatures =
            controller.signatures_from_key_and_cert(key_pem, cert_pem_or_der, password)?;
        Self::wrap(spec.uid3, &controller.with_signatures(signatures), data)
    }

    fn wrap(uid3: u32, controller: &SisController, data: SisData) -> Result<Vec<u8>> {
        Ok(Self::new(
            SisUid::new(uid3),
            SisCompressed::zlib(&controller.field().bytes())?,
            data,
        )
        .bytes())
    }
}
