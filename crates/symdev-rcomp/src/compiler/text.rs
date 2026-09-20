//! Text members: the six forms and their alignment (spec §3).

use symdev_core::{Error, Result};

use super::{RscResourceData, RssCompiler};
use crate::parser::RssTextForm;

impl RssCompiler {
    pub(super) fn text(
        &self,
        data: &mut RscResourceData,
        text: &[u32],
        form: RssTextForm,
        bits: u8,
    ) -> Result<()> {
        if bits == 8 {
            let bytes: Vec<u8> = text
                .iter()
                .map(|&c| {
                    u8::try_from(c).map_err(|_| Error::Other(format!("U+{c:04X} in 8-bit text")))
                })
                .collect::<Result<_>>()?;
            match form {
                RssTextForm::Counted => {
                    data.raw(&[bytes.len() as u8]);
                    data.raw(&bytes);
                }
                RssTextForm::Bare => data.raw(&bytes),
                RssTextForm::Terminated => {
                    data.raw(&bytes);
                    data.raw(&[0]);
                }
            }
            return Ok(());
        }
        let units: Vec<u16> = text
            .iter()
            .map(|&c| {
                u16::try_from(c).map_err(|_| {
                    Error::Other(format!("TODO: U+{c:X} outside the BMP (not observed)"))
                })
            })
            .collect::<Result<_>>()?;
        match form {
            RssTextForm::Counted => {
                // Truncated to 8 bits, unchecked, as rcomp writes it (spec §3.1).
                data.raw(&[units.len() as u8]);
                data.text16(&units);
            }
            RssTextForm::Bare => data.text16(&units),
            // The terminator belongs to the text, so it is compressed with it.
            RssTextForm::Terminated => {
                let mut units = units;
                units.push(0);
                data.text16(&units);
            }
        }
        Ok(())
    }
}
