//! Resource source statements (`NAME`, `UID2`/`UID3`, `STRUCT`, `RESOURCE`, `enum`,
//! `rls_*`) parsed from `.rpp` tokens.

mod ast;
mod value;

use symdev_core::{Error, Result};

use crate::lexer::{RssSpanned, RssToken};

pub use ast::{
    RssExpr, RssItem, RssMember, RssResource, RssStruct, RssStructValue, RssTextForm, RssTextPart,
    RssType, RssValue, RssWidth,
};

/// Parser over `.rpp` tokens.
pub struct RssParser {
    toks: Vec<RssSpanned>,
    at: usize,
}

impl RssParser {
    pub fn new(toks: Vec<RssSpanned>) -> Self {
        Self { toks, at: 0 }
    }

    pub fn items(mut self) -> Result<Vec<RssItem>> {
        let mut out = Vec::new();
        while self.at < self.toks.len() {
            if self.eat_punct(';') {
                continue;
            }
            out.push(self.item()?);
        }
        Ok(out)
    }

    fn error(&self, what: &str) -> Error {
        match self.toks.get(self.at).or(self.toks.last()) {
            Some(t) => Error::Other(format!("{}({}): {what}", t.file, t.line)),
            None => Error::Other(what.to_string()),
        }
    }

    fn peek(&self) -> Option<&RssToken> {
        self.toks.get(self.at).map(|t| &t.token)
    }

    fn peek_at(&self, n: usize) -> Option<&RssToken> {
        self.toks.get(self.at + n).map(|t| &t.token)
    }

    fn next(&mut self) -> Result<RssToken> {
        let t = self
            .toks
            .get(self.at)
            .map(|t| t.token.clone())
            .ok_or_else(|| self.error("unexpected end of file"))?;
        self.at += 1;
        Ok(t)
    }

    fn eat_punct(&mut self, c: char) -> bool {
        if self.peek() == Some(&RssToken::Punct(c)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn expect_punct(&mut self, c: char) -> Result<()> {
        if self.eat_punct(c) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{c}`, found {:?}", self.peek())))
        }
    }

    fn ident(&mut self) -> Result<String> {
        match self.next()? {
            RssToken::Ident(s) => Ok(s),
            other => {
                self.at -= 1;
                Err(self.error(&format!("expected a name, found {other:?}")))
            }
        }
    }

    fn peek_ident(&self) -> Option<&str> {
        match self.peek() {
            Some(RssToken::Ident(s)) => Some(s),
            _ => None,
        }
    }

    fn item(&mut self) -> Result<RssItem> {
        let keyword = self.ident()?;
        match keyword.as_str() {
            "NAME" => Ok(RssItem::Name(self.ident()?)),
            "UID2" => Ok(RssItem::Uid2(self.expr()?)),
            "UID3" => Ok(RssItem::Uid3(self.expr()?)),
            "CHARACTER_SET" => Ok(RssItem::CharacterSet(self.ident()?)),
            "STRUCT" => self.structure().map(RssItem::Struct),
            "RESOURCE" => self.resource().map(RssItem::Resource),
            "enum" | "ENUM" => self.enumeration().map(RssItem::Enum),
            "rls_string" | "rls_string8" | "rls_string16" | "rls_long" | "rls_word"
            | "rls_byte" | "rls_double" => {
                let name = self.ident()?;
                Ok(RssItem::Rls(name, self.value()?))
            }
            other => {
                self.at -= 1;
                Err(self.error(&format!("unknown statement `{other}`")))
            }
        }
    }

    fn width(&mut self) -> Option<RssWidth> {
        match self.peek_ident() {
            Some("BYTE") => {
                self.at += 1;
                Some(RssWidth::Byte)
            }
            Some("WORD") => {
                self.at += 1;
                Some(RssWidth::Word)
            }
            _ => None,
        }
    }

    fn structure(&mut self) -> Result<RssStruct> {
        let name = self.ident()?;
        let len_prefix = self.width();
        self.expect_punct('{')?;
        let mut members = Vec::new();
        while !self.eat_punct('}') {
            if self.eat_punct(';') {
                continue;
            }
            members.push(self.member()?);
        }
        Ok(RssStruct {
            name,
            len_prefix,
            members,
        })
    }

    fn member(&mut self) -> Result<RssMember> {
        let mut len_prefix = None;
        if self.peek_ident() == Some("LEN") {
            self.at += 1;
            len_prefix = Some(
                self.width()
                    .ok_or_else(|| self.error("LEN must be followed by BYTE or WORD"))?,
            );
        }
        let type_name = self.ident()?;
        let ty = Self::member_type(&type_name)
            .ok_or_else(|| self.error(&format!("unknown member type `{type_name}`")))?;
        let mut max_len = None;
        if self.eat_punct('<') {
            max_len = Some(self.expr()?);
            self.expect_punct('>')?;
        }
        let name = self.ident()?;
        if self.eat_punct('(') {
            max_len = Some(self.expr()?);
            self.expect_punct(')')?;
        }
        let mut array = None;
        if self.eat_punct('[') {
            if self.eat_punct(']') {
                array = Some(None);
            } else {
                array = Some(Some(self.expr()?));
                self.expect_punct(']')?;
            }
        }
        let default = if self.eat_punct('=') {
            Some(self.value()?)
        } else {
            None
        };
        self.expect_punct(';')?;
        Ok(RssMember {
            name,
            ty,
            max_len,
            array,
            len_prefix,
            default,
        })
    }

    fn member_type(name: &str) -> Option<RssType> {
        use RssTextForm::{Bare, Counted, Terminated};
        let text = |form, bits| RssType::Text { form, bits };
        Some(match name {
            "BYTE" => RssType::Byte,
            "WORD" => RssType::Word,
            "LONG" => RssType::Long,
            "DOUBLE" => RssType::Double,
            "LTEXT" | "LTEXT16" => text(Counted, 16),
            "LTEXT8" => text(Counted, 8),
            "BUF" | "BUF16" => text(Bare, 16),
            "BUF8" => text(Bare, 8),
            "TEXT" | "TEXT16" => text(Terminated, 16),
            "TEXT8" => text(Terminated, 8),
            "LINK" => RssType::Link,
            "LLINK" => RssType::Llink,
            "SRLINK" => RssType::Srlink,
            "STRUCT" => RssType::Struct,
            _ => return None,
        })
    }

    fn resource(&mut self) -> Result<RssResource> {
        let (file, line) = self
            .toks
            .get(self.at)
            .map(|t| (t.file.clone(), t.line))
            .unwrap_or_default();
        let struct_name = self.ident()?;
        let name = match self.peek() {
            Some(RssToken::Ident(_)) => Some(self.ident()?),
            _ => None,
        };
        let fields = self.fields()?;
        Ok(RssResource {
            value: RssStructValue {
                struct_name,
                fields,
            },
            name,
            file,
            line,
        })
    }

    /// `{ member = value; … }`
    fn fields(&mut self) -> Result<Vec<(String, RssValue)>> {
        self.expect_punct('{')?;
        let mut fields = Vec::new();
        while !self.eat_punct('}') {
            if self.eat_punct(';') {
                continue;
            }
            let name = self.ident()?;
            self.expect_punct('=')?;
            let value = self.value()?;
            fields.push((name, value));
            if !self.eat_punct(';') && self.peek() != Some(&RssToken::Punct('}')) {
                return Err(self.error("expected `;` after a member value"));
            }
        }
        Ok(fields)
    }

    fn enumeration(&mut self) -> Result<Vec<(String, Option<RssExpr>)>> {
        if self.peek_ident().is_some() {
            self.at += 1;
        }
        self.expect_punct('{')?;
        let mut out = Vec::new();
        while !self.eat_punct('}') {
            let name = self.ident()?;
            let value = if self.eat_punct('=') {
                Some(self.expr()?)
            } else {
                None
            };
            out.push((name, value));
            if !self.eat_punct(',') && self.peek() != Some(&RssToken::Punct('}')) {
                return Err(self.error("expected `,` or `}` in enum"));
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests;
