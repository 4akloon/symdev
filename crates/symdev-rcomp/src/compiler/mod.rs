//! Resource values → resource bytes (`rcomp -u`), from the experiment-56 goldens and
//! [rcomp-spec.md](../../../docs/research/rcomp-spec.md).

mod data;
mod eval;
mod layout;
mod text;

use std::collections::HashMap;

use symdev_core::{Error, Result};

use crate::parser::{RssItem, RssResource, RssStruct};

pub use data::{RscCompiled, RscCompiledResource, RscResourceData, RscSegment};

#[derive(Debug, Clone, PartialEq)]
enum RssConst {
    Int(i64),
    Real(f64),
    Text(Vec<u32>),
}

/// Compiles parsed items. Names are resolved in one table: enum values, `rls_*`
/// constants, then resource names (their ids).
pub struct RssCompiler {
    structs: HashMap<String, RssStruct>,
    consts: HashMap<String, RssConst>,
    resource_ids: HashMap<String, u32>,
}

impl RssCompiler {
    /// `NAME` value: base 27, `A`/`a` = 1 … `Z`/`z` = 26, digits 0 (experiment 56:
    /// `TEST` → `0x6120e`, `AB` → 29, `L10N` → `0x39ab2`).
    pub fn name_value(name: &str) -> Result<u32> {
        if name.chars().count() > 4 {
            return Err(Error::Other(format!(
                "NAME {name} is longer than four characters"
            )));
        }
        let mut value = 0u32;
        for c in name.chars() {
            let digit = match c {
                'A'..='Z' => c as u32 - 'A' as u32 + 1,
                'a'..='z' => c as u32 - 'a' as u32 + 1,
                _ => 0,
            };
            value = value * 27 + digit;
        }
        Ok(value)
    }

    pub fn compile(items: &[RssItem]) -> Result<RscCompiled> {
        let mut me = Self {
            structs: HashMap::new(),
            consts: HashMap::new(),
            resource_ids: HashMap::new(),
        };
        let mut name = None;
        let (mut uid2, mut uid3) = (None, None);
        let mut resources = Vec::new();
        for item in items {
            match item {
                RssItem::Name(n) => name = Some(Self::name_value(n)?),
                RssItem::Struct(s) => {
                    me.structs.insert(s.name.clone(), s.clone());
                }
                RssItem::Enum(values) => {
                    let mut next = 0i64;
                    for (n, e) in values {
                        let v = match e {
                            Some(e) => me.int(e)?,
                            None => next,
                        };
                        me.consts.insert(n.clone(), RssConst::Int(v));
                        next = v + 1;
                    }
                }
                RssItem::Rls(n, v) => {
                    let c = me.constant(v)?;
                    me.consts.insert(n.clone(), c);
                }
                RssItem::Uid2(e) => uid2 = Some(me.int(e)? as u32),
                RssItem::Uid3(e) => uid3 = Some(me.int(e)? as u32),
                // Handled by the lexer (RssCharset).
                RssItem::CharacterSet(_) => {}
                RssItem::Resource(r) => resources.push(r),
            }
        }
        let name_value = name.unwrap_or(0);
        for (i, r) in resources.iter().enumerate() {
            if let Some(n) = &r.name {
                me.resource_ids
                    .insert(n.clone(), (name_value << 12) | (i as u32 + 1));
            }
        }
        let mut out = Vec::new();
        for (i, r) in resources.iter().enumerate() {
            let id = (name_value << 12) | (i as u32 + 1);
            let data = me
                .resource(r, id)
                .map_err(|e| Error::Other(format!("{}({}): {e}", r.file, r.line)))?;
            out.push(RscCompiledResource {
                name: r.name.clone(),
                id,
                data,
            });
        }
        Ok(RscCompiled {
            uid2: uid2.unwrap_or(0),
            uid3: uid3.unwrap_or(name_value),
            uid3_from_name: uid3.is_none(),
            named: name.is_some(),
            resources: out,
        })
    }

    fn resource(&self, r: &RssResource, id: u32) -> Result<RscResourceData> {
        let mut data = RscResourceData::default();
        self.structure(&mut data, &r.value, id, true)?;
        Ok(data)
    }

    fn struct_def(&self, name: &str) -> Result<&RssStruct> {
        self.structs
            .get(name)
            .ok_or_else(|| Error::Other(format!("unknown STRUCT {name}")))
    }
}

#[cfg(test)]
mod spec_tests;
#[cfg(test)]
mod tests;
