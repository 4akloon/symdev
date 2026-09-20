//! Struct and member layout: declaration order, defaults, arrays, length prefixes,
//! text forms.

use symdev_core::{Error, Result};

use super::{RscResourceData, RscSegment, RssCompiler, RssConst};
use crate::parser::{RssExpr, RssMember, RssStruct, RssStructValue, RssType, RssValue, RssWidth};

impl RssCompiler {
    /// `rcomp` accepts `-2^(8k-1) ..= 2^(8k)-1` for a `k`-byte integer and rejects the
    /// rest rather than truncating (spec §5.1).
    fn fits(value: i64, bytes: u32) -> Result<i64> {
        let bits = 8 * i64::from(bytes);
        if value >= -(1i64 << (bits - 1)) && value < (1i64 << bits) {
            Ok(value)
        } else {
            Err(Error::Other(format!(
                "{value} does not fit in {bytes} byte(s)"
            )))
        }
    }

    /// A struct instance: members in declaration order, assigned value else default.
    /// `top`: the resource itself (no length prefix).
    pub(super) fn structure(
        &self,
        data: &mut RscResourceData,
        value: &RssStructValue,
        id: u32,
        top: bool,
    ) -> Result<()> {
        let def = self.struct_def(&value.struct_name)?;
        for (field, _) in &value.fields {
            if !def.members.iter().any(|m| &m.name == field) {
                return Err(Error::Other(format!("{} has no member {field}", def.name)));
            }
        }
        if def.len_prefix.is_some() && !top {
            return Err(Error::Other(format!(
                "TODO: length-prefixed nested list {} (spec §5.5: the source syntax was not \
                 observed, so there is no golden)",
                def.name
            )));
        }
        if let (Some(width), false) = (def.len_prefix, top) {
            // Padding stays relative to the resource: build in place, patch the
            // length afterwards.
            let prefix_at = data.len();
            data.raw(&match width {
                RssWidth::Byte => vec![0],
                RssWidth::Word => vec![0, 0],
            });
            let start = data.len();
            self.members(data, def, value, id)?;
            let len = data.len() - start;
            Self::patch_prefix(data, prefix_at, width, len)?;
            return Ok(());
        }
        self.members(data, def, value, id)
    }

    fn patch_prefix(
        data: &mut RscResourceData,
        at: usize,
        width: RssWidth,
        len: usize,
    ) -> Result<()> {
        let bytes = match width {
            RssWidth::Byte => vec![
                u8::try_from(len)
                    .map_err(|_| Error::Other(format!("BYTE-length struct of {len} bytes")))?,
            ],
            RssWidth::Word => u16::try_from(len)
                .map_err(|_| Error::Other(format!("WORD-length struct of {len} bytes")))?
                .to_le_bytes()
                .to_vec(),
        };
        let mut offset = 0;
        for seg in &mut data.segments {
            let n = match seg {
                RscSegment::Raw(b) => b.len(),
                RscSegment::Pad => 1,
                RscSegment::Text(t) => 2 * t.len(),
            };
            if (offset..offset + n).contains(&at)
                && let RscSegment::Raw(b) = seg
                && let Some(slot) = b.get_mut(at - offset..at - offset + bytes.len())
            {
                slot.copy_from_slice(&bytes);
                return Ok(());
            }
            offset += n;
        }
        Err(Error::Other("struct length prefix lost".into()))
    }

    fn members(
        &self,
        data: &mut RscResourceData,
        def: &RssStruct,
        value: &RssStructValue,
        id: u32,
    ) -> Result<()> {
        for m in &def.members {
            let assigned = value
                .fields
                .iter()
                .rev()
                .find(|(f, _)| f == &m.name)
                .map(|(_, v)| v);
            let v = assigned.or(m.default.as_ref());
            self.member(data, m, v, id)
                .map_err(|e| Error::Other(format!("{}.{}: {e}", def.name, m.name)))?;
        }
        Ok(())
    }

    fn member(
        &self,
        data: &mut RscResourceData,
        m: &RssMember,
        value: Option<&RssValue>,
        id: u32,
    ) -> Result<()> {
        match &m.array {
            None => self.scalar(data, m, value, id),
            Some(fixed) => {
                let items: Vec<&RssValue> = match value {
                    None => Vec::new(),
                    Some(RssValue::List(items)) => items.iter().collect(),
                    Some(other) => {
                        return Err(Error::Other(format!(
                            "array needs `{{ … }}`, got {other:?}"
                        )));
                    }
                };
                match fixed {
                    Some(n) => {
                        let n = self.int(n)? as usize;
                        if items.len() > n {
                            return Err(Error::Other(format!(
                                "{} items for a [{n}] array",
                                items.len()
                            )));
                        }
                    }
                    None => {
                        let count = items.len();
                        match m.len_prefix.unwrap_or(RssWidth::Word) {
                            RssWidth::Byte => data.raw(&[u8::try_from(count).map_err(|_| {
                                Error::Other(format!("{count} items for LEN BYTE"))
                            })?]),
                            RssWidth::Word => data.raw(
                                &u16::try_from(count)
                                    .map_err(|_| {
                                        Error::Other(format!("{count} items for LEN WORD"))
                                    })?
                                    .to_le_bytes(),
                            ),
                        }
                    }
                }
                for item in items {
                    self.scalar(data, m, Some(item), id)?;
                }
                Ok(())
            }
        }
    }

    fn scalar(
        &self,
        data: &mut RscResourceData,
        m: &RssMember,
        value: Option<&RssValue>,
        id: u32,
    ) -> Result<()> {
        match m.ty {
            RssType::Byte => data.raw(&[Self::fits(self.int_value(value)?, 1)? as u8]),
            RssType::Word => {
                data.raw(&(Self::fits(self.int_value(value)?, 2)? as u16).to_le_bytes());
            }
            RssType::Long => {
                data.raw(&(Self::fits(self.int_value(value)?, 4)? as u32).to_le_bytes());
            }
            RssType::Double => {
                let v = match value {
                    None => 0.0,
                    Some(v) => match self.constant(v)? {
                        RssConst::Real(r) => r,
                        RssConst::Int(i) => i as f64,
                        RssConst::Text(_) => return Err(Error::Other("text for DOUBLE".into())),
                    },
                };
                data.raw(&v.to_le_bytes());
            }
            RssType::Link | RssType::Llink if value.is_none() => {}
            RssType::Link => data.raw(&(self.link(value)? as u16).to_le_bytes()),
            RssType::Llink => data.raw(&self.link(value)?.to_le_bytes()),
            RssType::Srlink => data.raw(&id.to_le_bytes()),
            RssType::Text { form, bits } => {
                let text = match value {
                    None => Vec::new(),
                    // An undefined name where text is expected is taken as its own
                    // spelling (experiment 56: helloworldbasic's missing `.rls` names
                    // appear verbatim in the Wine `.rsc`).
                    Some(RssValue::Expr(RssExpr::Name(n)))
                        if !self.consts.contains_key(n) && !self.resource_ids.contains_key(n) =>
                    {
                        n.chars().map(u32::from).collect()
                    }
                    Some(v) => match self.constant(v)? {
                        RssConst::Text(t) => t,
                        other => {
                            return Err(Error::Other(format!("number {other:?} for a text")));
                        }
                    },
                };
                if let Some(max) = &m.max_len {
                    let max = self.int(max)? as usize;
                    if text.len() > max {
                        return Err(Error::Other(format!(
                            "text of {} characters, limit {max}",
                            text.len()
                        )));
                    }
                }
                self.text(data, &text, form, bits)?;
            }
            RssType::Struct => match value {
                Some(RssValue::Struct(s)) => self.structure(data, s, id, false)?,
                // `caption_and_icon = { CAPTION_AND_ICON_INFO { … } };` (SDK examples).
                Some(RssValue::List(items)) if items.len() == 1 => {
                    self.scalar(data, m, Some(&items[0]), id)?;
                }
                None => {
                    return Err(Error::Other(
                        "TODO: STRUCT member without a value (not observed)".into(),
                    ));
                }
                Some(other) => {
                    return Err(Error::Other(format!("STRUCT member given {other:?}")));
                }
            },
        }
        Ok(())
    }
}
