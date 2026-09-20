//! Name resolution and integer / text / real constants.

use symdev_core::{Error, Result};

use super::{RssCompiler, RssConst};
use crate::parser::{RssExpr, RssTextPart, RssValue};

impl RssCompiler {
    pub(super) fn link(&self, value: Option<&RssValue>) -> Result<u32> {
        match value {
            None => Ok(0),
            Some(RssValue::Expr(RssExpr::Name(n))) if self.resource_ids.contains_key(n) => {
                Ok(self.resource_ids[n])
            }
            Some(v) => Ok(self.int_value(Some(v))? as u32),
        }
    }

    pub(super) fn int_value(&self, value: Option<&RssValue>) -> Result<i64> {
        match value {
            None => Ok(0),
            Some(RssValue::Expr(e)) => self.int(e),
            Some(v) => match self.constant(v)? {
                RssConst::Int(i) => Ok(i),
                other => Err(Error::Other(format!(
                    "{other:?} where a number is expected"
                ))),
            },
        }
    }

    pub(super) fn constant(&self, v: &RssValue) -> Result<RssConst> {
        match v {
            RssValue::Expr(RssExpr::Name(n)) => self
                .consts
                .get(n)
                .cloned()
                .or_else(|| {
                    self.resource_ids
                        .get(n)
                        .map(|&id| RssConst::Int(i64::from(id)))
                })
                .ok_or_else(|| Error::Other(format!("unknown name {n}"))),
            RssValue::Expr(RssExpr::Real(r)) => Ok(RssConst::Real(*r)),
            RssValue::Expr(RssExpr::Neg(inner)) if matches!(**inner, RssExpr::Real(_)) => {
                match **inner {
                    RssExpr::Real(r) => Ok(RssConst::Real(-r)),
                    _ => Err(Error::Other("negated real expected".into())),
                }
            }
            RssValue::Expr(e) => self.int(e).map(RssConst::Int),
            RssValue::Text(parts) => {
                let mut out = Vec::new();
                for p in parts {
                    match p {
                        RssTextPart::Literal(s) => out.extend_from_slice(s),
                        RssTextPart::Code(e) => out.push(self.int(e)? as u32),
                    }
                }
                Ok(RssConst::Text(out))
            }
            RssValue::List(_) | RssValue::Struct(_) => Err(Error::Other(
                "a list or struct where a single value is expected".into(),
            )),
        }
    }

    pub(super) fn int(&self, e: &RssExpr) -> Result<i64> {
        Ok(match e {
            RssExpr::Int(i) => *i,
            RssExpr::Char(c) => i64::from(*c),
            RssExpr::Real(r) => {
                return Err(Error::Other(format!(
                    "real {r} where an integer is expected"
                )));
            }
            RssExpr::Name(n) => match self.consts.get(n) {
                Some(RssConst::Int(i)) => *i,
                Some(other) => {
                    return Err(Error::Other(format!("{n} is {other:?}, not a number")));
                }
                None => match self.resource_ids.get(n) {
                    Some(&id) => i64::from(id),
                    None => return Err(Error::Other(format!("unknown name {n}"))),
                },
            },
            RssExpr::Neg(x) => -self.int(x)?,
            RssExpr::Not(x) => !self.int(x)?,
            RssExpr::Binary(op, a, b) => {
                let (a, b) = (self.int(a)?, self.int(b)?);
                match op {
                    '|' => a | b,
                    '&' => a & b,
                    '+' => a.wrapping_add(b),
                    '-' => a.wrapping_sub(b),
                    '*' => a.wrapping_mul(b),
                    '/' => {
                        if b == 0 {
                            return Err(Error::Other("division by zero".into()));
                        }
                        a / b
                    }
                    other => return Err(Error::Other(format!("operator {other}"))),
                }
            }
        })
    }
}
