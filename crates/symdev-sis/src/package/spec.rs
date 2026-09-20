//! `SisUnsignedSpec`: the unsigned-SIS controller and data blob builder.
use sha1::{Digest, Sha1};
use symdev_core::{Error, Result};

use super::pkg_file::SisPkgFile;
use crate::{
    SisArray, SisCompressed, SisController, SisData, SisData31, SisData32, SisDateTime, SisEncode,
    SisField, SisFile, SisFiles, SisHash, SisInfo, SisLanguage, SisLanguages, SisPkgUid,
    SisProduct, SisProductVersion, SisProducts, SisString, SisU32, SisVersion, SisWord41, SisWords,
    SisWords16, SisWords19,
};

pub struct SisUnsignedSpec<'a> {
    pub name: &'a str,
    pub uid3: u32,
    pub version: (u32, u32, u32),
    pub vendor: &'a str,
    pub vendor_localized: &'a str,
    pub exe: &'a [u8],
    pub capabilities: &'a [String],
    pub datetime: SisDateTime,
    /// Non-EXE files in `.pkg` order (experiment 51: SIS keeps that order).
    pub files: &'a [SisPkgFile<'a>],
}

impl SisUnsignedSpec<'_> {
    /// File description (controller) and its data blob, sharing one compression choice.
    fn install_file(
        dest: String,
        data: &[u8],
        caps: Option<SisWord41>,
        idx: u32,
    ) -> Result<(SisField, SisField)> {
        let blob = SisCompressed::smallest(data)?;
        let stored = blob.data.len() as u32;
        let size = data.len() as u32;
        let digest: [u8; 20] = Sha1::digest(data).into();
        let file = SisFile::new(
            SisString::new(dest),
            SisString::new(""),
            caps,
            SisHash::new([1, 0x25, 0x14], digest),
            SisString::new(""),
            [stored, 0, size, 0, idx],
        )
        .field();
        Ok((file, SisData32::new(blob).field()))
    }

    pub(super) fn parts(&self) -> Result<(SisController, SisData)> {
        let caps = SisWord41::from_capabilities(self.capabilities)?;
        let (major, minor, build) = self.version;
        // makesis writes type 41 only when the EXE has capabilities (experiment 51).
        let caps = (caps.value != 0).then_some(caps);
        let (exe_file, exe_blob) = Self::install_file(
            format!("!:\\sys\\bin\\{}.exe", self.name),
            self.exe,
            caps,
            0,
        )?;
        let mut files = vec![exe_file];
        let mut blobs = vec![exe_blob];
        for (idx, extra) in self.files.iter().enumerate() {
            let caps = extra.e32_capabilities().filter(|c| c.value != 0);
            let (file, blob) =
                Self::install_file(extra.dest.to_string(), extra.data, caps, idx as u32 + 1)?;
            files.push(file);
            blobs.push(blob);
        }
        let controller = SisController::new(
            SisInfo::new(
                SisPkgUid::new(self.uid3),
                SisString::new(self.vendor),
                SisArray::new(vec![SisString::new(self.name).field()]),
                SisArray::new(vec![SisString::new(self.vendor_localized).field()]),
                SisVersion::new(major, minor, build),
                self.datetime,
            ),
            SisWords16::new(SisWords::new(vec![0x21])), // ponytail: recorded TYPE=SA &EN one-file words; derive when pkg grammar grows
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
                SisArray::new(files),
                SisWords::new(vec![0x0d]),
                SisWords::new(vec![0x1a]),
            ),
            SisU32::new(0),
        );
        let data = SisData::new(SisArray::new(vec![
            SisData31::new(SisArray::new(blobs)).field(),
        ]));
        Ok((controller, data))
    }
}

impl SisWord41 {
    fn from_capabilities(caps: &[String]) -> Result<Self> {
        let bits = symdev_core::Capabilities::from_names(caps)?.bits();
        let word = u32::try_from(bits)
            .map_err(|_| Error::Other(format!("SIS type-41 word cannot hold {bits:#x}")))?;
        Ok(Self::new(word))
    }
}
