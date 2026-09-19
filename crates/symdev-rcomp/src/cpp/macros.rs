//! `#define` table and macro expansion (object- and function-like, `#`, `##`).

use std::collections::{HashMap, HashSet};

use symdev_core::{Error, Result};

use super::tokens::CppToken;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CppMacro {
    /// `None`: object-like.
    params: Option<Vec<String>>,
    body: Vec<CppToken>,
}

#[derive(Debug, Default)]
pub struct CppMacros {
    map: HashMap<String, CppMacro>,
}

impl CppMacros {
    /// `NAME=value` or `NAME` (value 1), as `cpp -D`.
    pub fn predefine(&mut self, spec: &str) {
        let (name, value) = spec.split_once('=').unwrap_or((spec, "1"));
        self.map.insert(
            name.to_string(),
            CppMacro {
                params: None,
                body: super::tokens::CppSource::tokens(value),
            },
        );
    }

    pub fn is_defined(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn undef(&mut self, name: &str) {
        self.map.remove(name);
    }

    /// The tokens after `#define`.
    pub fn define(&mut self, rest: &[CppToken]) -> Result<()> {
        let mut i = skip_space(rest, 0);
        let Some(CppToken::Ident(name)) = rest.get(i) else {
            return Err(Error::Other("#define without a name".into()));
        };
        i += 1;
        let mut params = None;
        // Function-like only when `(` follows the name with no space.
        if rest.get(i).map(CppToken::text) == Some("(") {
            let mut list = Vec::new();
            i += 1;
            loop {
                i = skip_space(rest, i);
                match rest.get(i) {
                    Some(CppToken::Ident(p)) => list.push(p.clone()),
                    Some(t) if t.text() == ")" => {
                        i += 1;
                        break;
                    }
                    other => {
                        return Err(Error::Other(format!(
                            "#define {name}: bad parameter list at {other:?}"
                        )));
                    }
                }
                i = skip_space(rest, i + 1);
                match rest.get(i).map(CppToken::text) {
                    Some(",") => i += 1,
                    Some(")") => {
                        i += 1;
                        break;
                    }
                    other => {
                        return Err(Error::Other(format!(
                            "#define {name}: bad parameter list at {other:?}"
                        )));
                    }
                }
            }
            params = Some(list);
        }
        let mut body: Vec<CppToken> = rest[i..].to_vec();
        while body.first().is_some_and(CppToken::is_space) {
            body.remove(0);
        }
        while body.last().is_some_and(CppToken::is_space) {
            body.pop();
        }
        self.map.insert(name.clone(), CppMacro { params, body });
        Ok(())
    }

    pub fn expand(&self, tokens: &[CppToken]) -> Result<Vec<CppToken>> {
        self.expand_with(tokens, &HashSet::new(), 0)
    }

    fn expand_with(
        &self,
        tokens: &[CppToken],
        disabled: &HashSet<String>,
        depth: usize,
    ) -> Result<Vec<CppToken>> {
        if depth > 200 {
            return Err(Error::Other("macro expansion too deep".into()));
        }
        let mut out = Vec::with_capacity(tokens.len());
        let mut i = 0;
        while i < tokens.len() {
            let tok = &tokens[i];
            let CppToken::Ident(name) = tok else {
                out.push(tok.clone());
                i += 1;
                continue;
            };
            let Some(m) = self.map.get(name).filter(|_| !disabled.contains(name)) else {
                out.push(tok.clone());
                i += 1;
                continue;
            };
            let mut inner = disabled.clone();
            inner.insert(name.clone());
            match &m.params {
                None => {
                    out.push(CppToken::Space);
                    out.extend(self.expand_with(&m.body, &inner, depth + 1)?);
                    out.push(CppToken::Space);
                    i += 1;
                }
                Some(params) => {
                    let mut j = i + 1;
                    while tokens
                        .get(j)
                        .is_some_and(|t| matches!(t, CppToken::Space | CppToken::Newline))
                    {
                        j += 1;
                    }
                    if tokens.get(j).map(CppToken::text) != Some("(") {
                        out.push(tok.clone());
                        i += 1;
                        continue;
                    }
                    let (args, end) = Self::arguments(tokens, j)?;
                    if args.len() != params.len() && !(params.is_empty() && args.len() == 1) {
                        return Err(Error::Other(format!(
                            "macro {name} takes {} arguments, got {}",
                            params.len(),
                            args.len()
                        )));
                    }
                    let body = self.substitute(m, params, &args, &inner, depth)?;
                    out.push(CppToken::Space);
                    out.extend(self.expand_with(&body, &inner, depth + 1)?);
                    out.push(CppToken::Space);
                    i = end;
                }
            }
        }
        Ok(out)
    }

    /// Arguments of the invocation whose `(` is at `open`; returns them and the index
    /// after `)`.
    fn arguments(tokens: &[CppToken], open: usize) -> Result<(Vec<Vec<CppToken>>, usize)> {
        let mut args = vec![Vec::new()];
        let mut depth = 0;
        let mut i = open + 1;
        while let Some(t) = tokens.get(i) {
            match t.text() {
                "(" => depth += 1,
                ")" if depth == 0 => {
                    let args = args.into_iter().map(|a| trim(&a)).collect();
                    return Ok((args, i + 1));
                }
                ")" => depth -= 1,
                "," if depth == 0 => {
                    args.push(Vec::new());
                    i += 1;
                    continue;
                }
                _ => {}
            }
            let t = if matches!(t, CppToken::Newline) {
                CppToken::Space
            } else {
                t.clone()
            };
            if let Some(last) = args.last_mut() {
                last.push(t);
            }
            i += 1;
        }
        Err(Error::Other("unterminated macro arguments".into()))
    }

    fn substitute(
        &self,
        m: &CppMacro,
        params: &[String],
        args: &[Vec<CppToken>],
        disabled: &HashSet<String>,
        depth: usize,
    ) -> Result<Vec<CppToken>> {
        let arg = |t: &CppToken| match t {
            CppToken::Ident(n) => params.iter().position(|p| p == n),
            _ => None,
        };
        let mut out: Vec<CppToken> = Vec::new();
        let mut i = 0;
        while i < m.body.len() {
            let t = &m.body[i];
            if t.text() == "#" {
                let j = skip_space(&m.body, i + 1);
                if let Some(k) = m.body.get(j).and_then(arg) {
                    let text: String = args[k].iter().map(CppToken::text).collect();
                    out.push(CppToken::Other(format!(
                        "\"{}\"",
                        text.replace('"', "\\\"")
                    )));
                    i = j + 1;
                    continue;
                }
            }
            let next = skip_space(&m.body, i + 1);
            let pasted = m.body.get(next).map(CppToken::text) == Some("##");
            let before_paste = out.last().is_some_and(|l: &CppToken| l.text() == "##");
            match arg(t) {
                // Operands of `##` are not expanded.
                Some(k) if pasted || before_paste => out.extend(args[k].iter().cloned()),
                Some(k) => out.extend(self.expand_with(&args[k], disabled, depth + 1)?),
                None => out.push(t.clone()),
            }
            i += 1;
        }
        Ok(Self::paste(out))
    }

    /// Join the tokens around each `##`.
    fn paste(tokens: Vec<CppToken>) -> Vec<CppToken> {
        let mut out: Vec<CppToken> = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            if tokens[i].text() == "##" {
                while out.last().is_some_and(CppToken::is_space) {
                    out.pop();
                }
                let j = skip_space(&tokens, i + 1);
                if let (Some(left), Some(right)) = (out.pop(), tokens.get(j)) {
                    let joined = format!("{}{}", left.text(), right.text());
                    let first = joined.chars().next().unwrap_or(' ');
                    out.push(if first.is_ascii_alphabetic() || first == '_' {
                        CppToken::Ident(joined)
                    } else {
                        CppToken::Other(joined)
                    });
                }
                i = j + 1;
                continue;
            }
            out.push(tokens[i].clone());
            i += 1;
        }
        out
    }
}

fn skip_space(tokens: &[CppToken], mut i: usize) -> usize {
    while tokens.get(i).is_some_and(CppToken::is_space) {
        i += 1;
    }
    i
}

fn trim(tokens: &[CppToken]) -> Vec<CppToken> {
    let start = skip_space(tokens, 0);
    let mut end = tokens.len();
    while end > start && tokens[end - 1].is_space() {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

#[cfg(test)]
mod tests;
