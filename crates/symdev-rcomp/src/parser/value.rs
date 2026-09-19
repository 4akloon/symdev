//! Values (`{ … }` lists, struct initialisers, texts) and integer expressions.

use symdev_core::Result;

use super::RssParser;
use super::ast::{RssExpr, RssStructValue, RssTextPart, RssValue};
use crate::lexer::RssToken;

impl RssParser {
    pub(super) fn value(&mut self) -> Result<RssValue> {
        match self.peek() {
            Some(RssToken::Punct('{')) => {
                self.at += 1;
                let mut items = Vec::new();
                while !self.eat_punct('}') {
                    items.push(self.value()?);
                    if !self.eat_punct(',') && self.peek() != Some(&RssToken::Punct('}')) {
                        return Err(self.error("expected `,` or `}` in a list"));
                    }
                }
                Ok(RssValue::List(items))
            }
            Some(RssToken::Str(_)) | Some(RssToken::Punct('<')) => self.text().map(RssValue::Text),
            Some(RssToken::Ident(_)) if self.peek_at(1) == Some(&RssToken::Punct('{')) => {
                let struct_name = self.ident()?;
                let fields = self.fields()?;
                Ok(RssValue::Struct(RssStructValue {
                    struct_name,
                    fields,
                }))
            }
            _ => self.expr().map(RssValue::Expr),
        }
    }

    /// `"a" <0x2029> "b"`: literals and character codes, concatenated.
    fn text(&mut self) -> Result<Vec<RssTextPart>> {
        let mut parts = Vec::new();
        loop {
            match self.peek() {
                Some(RssToken::Str(s)) => {
                    parts.push(RssTextPart::Literal(s.clone()));
                    self.at += 1;
                }
                Some(RssToken::Punct('<')) => {
                    self.at += 1;
                    parts.push(RssTextPart::Code(self.expr()?));
                    self.expect_punct('>')?;
                }
                _ => return Ok(parts),
            }
        }
    }

    pub(super) fn expr(&mut self) -> Result<RssExpr> {
        self.binary(0)
    }

    fn precedence(c: char) -> Option<u8> {
        match c {
            '|' => Some(1),
            '&' => Some(2),
            '+' | '-' => Some(3),
            '*' | '/' => Some(4),
            _ => None,
        }
    }

    fn binary(&mut self, min: u8) -> Result<RssExpr> {
        let mut left = self.unary()?;
        while let Some(RssToken::Punct(op)) = self.peek() {
            let op = *op;
            let Some(prec) = Self::precedence(op) else {
                break;
            };
            if prec < min {
                break;
            }
            self.at += 1;
            let right = self.binary(prec + 1)?;
            left = RssExpr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<RssExpr> {
        match self.next()? {
            RssToken::Int(n) => Ok(RssExpr::Int(n)),
            RssToken::Real(r) => Ok(RssExpr::Real(r)),
            RssToken::Char(c) => Ok(RssExpr::Char(c)),
            RssToken::Ident(s) => Ok(RssExpr::Name(s)),
            RssToken::Punct('-') => Ok(RssExpr::Neg(Box::new(self.unary()?))),
            RssToken::Punct('+') => self.unary(),
            RssToken::Punct('~') => Ok(RssExpr::Not(Box::new(self.unary()?))),
            RssToken::Punct('(') => {
                let e = self.expr()?;
                self.expect_punct(')')?;
                Ok(e)
            }
            other => {
                self.at -= 1;
                Err(self.error(&format!("expected a value, found {other:?}")))
            }
        }
    }
}
