//! `#if` / `#elif` expressions: `defined`, integer arithmetic, comparisons, logic.

use symdev_core::{Error, Result};

use super::macros::CppMacros;
use super::tokens::CppToken;

pub struct CppCondition;

impl CppCondition {
    /// Evaluate the tokens after `#if`: `defined X` / `defined(X)` first, then macro
    /// expansion; names left over are 0 (C rules, e.g. `#elif LANGUAGE_01`).
    pub fn eval(tokens: &[CppToken], macros: &CppMacros) -> Result<bool> {
        let mut resolved = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if let CppToken::Ident(n) = &tokens[i]
                && n == "defined"
            {
                let mut j = next_non_space(tokens, i + 1);
                let paren = tokens.get(j).map(CppToken::text) == Some("(");
                if paren {
                    j = next_non_space(tokens, j + 1);
                }
                let Some(CppToken::Ident(name)) = tokens.get(j) else {
                    return Err(Error::Other("`defined` without a name".into()));
                };
                let value = if macros.is_defined(name) { "1" } else { "0" };
                resolved.push(CppToken::Other(value.into()));
                i = j + 1;
                if paren {
                    i = next_non_space(tokens, i) + 1;
                }
                continue;
            }
            resolved.push(tokens[i].clone());
            i += 1;
        }
        let expanded = macros.expand(&resolved)?;
        let words: Vec<String> = expanded
            .iter()
            .filter(|t| !matches!(t, CppToken::Space | CppToken::Newline))
            .map(|t| t.text().to_string())
            .collect();
        let mut p = Parser { words, at: 0 };
        let v = p.ternary()?;
        if p.at != p.words.len() {
            return Err(Error::Other(format!(
                "#if: unexpected `{}`",
                p.words[p.at..].join(" ")
            )));
        }
        Ok(v != 0)
    }
}

fn next_non_space(tokens: &[CppToken], mut i: usize) -> usize {
    while tokens.get(i).is_some_and(CppToken::is_space) {
        i += 1;
    }
    i
}

struct Parser {
    words: Vec<String>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&str> {
        self.words.get(self.at).map(String::as_str)
    }

    fn eat(&mut self, w: &str) -> bool {
        if self.peek() == Some(w) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn ternary(&mut self) -> Result<i64> {
        let c = self.binary(0)?;
        if self.eat("?") {
            let a = self.ternary()?;
            if !self.eat(":") {
                return Err(Error::Other("#if: `?` without `:`".into()));
            }
            let b = self.ternary()?;
            return Ok(if c != 0 { a } else { b });
        }
        Ok(c)
    }

    fn precedence(op: &str) -> Option<u8> {
        Some(match op {
            "||" => 1,
            "&&" => 2,
            "|" => 3,
            "^" => 4,
            "&" => 5,
            "==" | "!=" => 6,
            "<" | ">" | "<=" | ">=" => 7,
            "<<" | ">>" => 8,
            "+" | "-" => 9,
            "*" | "/" | "%" => 10,
            _ => return None,
        })
    }

    fn binary(&mut self, min: u8) -> Result<i64> {
        let mut left = self.unary()?;
        while let Some(op) = self.peek().map(str::to_string) {
            let Some(prec) = Self::precedence(&op) else {
                break;
            };
            if prec < min {
                break;
            }
            self.at += 1;
            let right = self.binary(prec + 1)?;
            left = match op.as_str() {
                "||" => i64::from(left != 0 || right != 0),
                "&&" => i64::from(left != 0 && right != 0),
                "|" => left | right,
                "^" => left ^ right,
                "&" => left & right,
                "==" => i64::from(left == right),
                "!=" => i64::from(left != right),
                "<" => i64::from(left < right),
                ">" => i64::from(left > right),
                "<=" => i64::from(left <= right),
                ">=" => i64::from(left >= right),
                "<<" => left.wrapping_shl(right as u32),
                ">>" => left.wrapping_shr(right as u32),
                "+" => left.wrapping_add(right),
                "-" => left.wrapping_sub(right),
                "*" => left.wrapping_mul(right),
                "/" | "%" if right == 0 => {
                    return Err(Error::Other("#if: division by zero".into()));
                }
                "/" => left / right,
                _ => left % right,
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<i64> {
        let Some(w) = self.peek().map(str::to_string) else {
            return Err(Error::Other("#if: missing operand".into()));
        };
        self.at += 1;
        match w.as_str() {
            "!" => Ok(i64::from(self.unary()? == 0)),
            "~" => Ok(!self.unary()?),
            "-" => Ok(self.unary()?.wrapping_neg()),
            "+" => self.unary(),
            "(" => {
                let v = self.ternary()?;
                if !self.eat(")") {
                    return Err(Error::Other("#if: missing `)`".into()));
                }
                Ok(v)
            }
            _ => Self::number(&w),
        }
    }

    fn number(w: &str) -> Result<i64> {
        let first = w.chars().next().unwrap_or(' ');
        if first.is_ascii_alphabetic() || first == '_' {
            return Ok(0);
        }
        if let Some(c) = w.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
            return c
                .chars()
                .next()
                .map(|c| c as i64)
                .ok_or_else(|| Error::Other("#if: empty character constant".into()));
        }
        let digits = w.trim_end_matches(['u', 'U', 'l', 'L']);
        let parsed = if let Some(hex) = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
        {
            i64::from_str_radix(hex, 16)
        } else if digits.len() > 1 && digits.starts_with('0') {
            i64::from_str_radix(&digits[1..], 8)
        } else {
            digits.parse()
        };
        parsed.map_err(|_| Error::Other(format!("#if: bad number {w}")))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tokens::CppSource;
    use super::*;

    fn eval(defs: &[&str], expr: &str) -> bool {
        let mut m = CppMacros::default();
        for d in defs {
            m.define(&CppSource::tokens(d)).unwrap();
        }
        CppCondition::eval(&CppSource::tokens(expr), &m).unwrap()
    }

    #[test]
    fn evaluates_sdk_conditions() {
        assert!(eval(&[], "!defined(__UIKON_HRH__)"));
        assert!(!eval(&["__APPINFO_RH__"], "!defined __APPINFO_RH__"));
        assert!(!eval(&[], "LANGUAGE_01"));
        assert!(eval(&["V 3"], "V >= 2 && (V & 1) == 1"));
        assert!(eval(&[], "0x10 == 16 ? 1 : 0"));
    }
}
