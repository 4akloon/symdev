//! Native replacement for the SDK `cpp.exe -nostdinc -undef -C -D_UNICODE` step before
//! `rcomp` (experiment 9 argv): includes, macros and conditionals, with `# line
//! "file"` markers so `rcomp` errors point at the right source.

mod cond;
mod macros;
mod tokens;

use std::path::{Component, Path, PathBuf};

use symdev_core::{Error, Result};

use cond::CppCondition;
use macros::CppMacros;
use tokens::{CppSource, CppToken};

/// One level of `#if` nesting.
struct CppBranch {
    /// Lines in this branch are emitted.
    active: bool,
    /// Some branch of this `#if` was taken (later `#elif`/`#else` are skipped).
    taken: bool,
    /// The enclosing level was active.
    outer: bool,
}

pub struct RssPreprocessor {
    include_dirs: Vec<PathBuf>,
    macros: CppMacros,
    out: String,
    depth: usize,
}

impl RssPreprocessor {
    /// `-I` directories in order; `_UNICODE` is predefined as the SDK recipe does.
    pub fn new(include_dirs: &[PathBuf]) -> Self {
        let mut macros = CppMacros::default();
        macros.predefine("_UNICODE");
        Self {
            include_dirs: include_dirs.to_vec(),
            macros,
            out: String::new(),
            depth: 0,
        }
    }

    /// Preprocessed text of `source` (Latin-1 in, Latin-1 out as bytes).
    pub fn run(mut self, source: &Path) -> Result<Vec<u8>> {
        self.file(source)?;
        Ok(self.out.chars().map(|c| c as u32 as u8).collect())
    }

    fn marker(&mut self, path: &Path, line: usize, flag: &str) {
        self.out
            .push_str(&format!("# {line} \"{}\"{flag}\n", path.display()));
    }

    fn file(&mut self, path: &Path) -> Result<()> {
        if self.depth > 64 {
            return Err(Error::Other(format!(
                "#include nested too deep at {}",
                path.display()
            )));
        }
        let bytes = std::fs::read(path)
            .map_err(|e| Error::Other(format!("read {}: {e}", path.display())))?;
        let text = CppSource::clean(&bytes);
        self.marker(path, 1, if self.depth == 0 { "" } else { " 1" });
        let mut stack: Vec<CppBranch> = Vec::new();
        let mut block: Vec<CppToken> = Vec::new();
        for (index, line) in text.split('\n').enumerate() {
            let active = stack.last().is_none_or(|b| b.active);
            let trimmed = line.trim_start();
            if !trimmed.starts_with('#') {
                if active {
                    block.extend(CppSource::tokens(line));
                }
                block.push(CppToken::Newline);
                continue;
            }
            self.flush(&mut block)?;
            let toks = CppSource::tokens(trimmed);
            let mut i = 1;
            while toks.get(i).is_some_and(CppToken::is_space) {
                i += 1;
            }
            let name = toks.get(i).map(CppToken::text).unwrap_or("");
            let rest = toks.get(i + 1..).unwrap_or(&[]);
            let at = |e: Error| Error::Other(format!("{}:{}: {e}", path.display(), index + 1));
            match name {
                "if" | "ifdef" | "ifndef" => {
                    let cond = active && self.condition(name, rest).map_err(at)?;
                    stack.push(CppBranch {
                        active: cond,
                        taken: cond,
                        outer: active,
                    });
                }
                "elif" => {
                    let b = stack
                        .last()
                        .ok_or_else(|| at(Error::Other("#elif without #if".into())))?;
                    let (outer, taken) = (b.outer, b.taken);
                    let cond = outer && !taken && self.condition("if", rest).map_err(at)?;
                    if let Some(b) = stack.last_mut() {
                        b.active = cond;
                        b.taken |= cond;
                    }
                }
                "else" => {
                    let b = stack
                        .last_mut()
                        .ok_or_else(|| at(Error::Other("#else without #if".into())))?;
                    b.active = b.outer && !b.taken;
                    b.taken = true;
                }
                "endif" => {
                    stack
                        .pop()
                        .ok_or_else(|| at(Error::Other("#endif without #if".into())))?;
                }
                _ if !active => {}
                "define" => self.macros.define(rest).map_err(at)?,
                "undef" => {
                    if let Some(CppToken::Ident(n)) = rest.iter().find(|t| !t.is_space()) {
                        self.macros.undef(n);
                    }
                }
                "include" => {
                    let target = self.include_target(path, rest).map_err(at)?;
                    self.depth += 1;
                    let result = self.file(&target);
                    self.depth -= 1;
                    result?;
                    self.marker(path, index + 2, " 2");
                    continue;
                }
                "" | "pragma" => {}
                "error" => {
                    let msg: String = rest.iter().map(CppToken::text).collect();
                    return Err(at(Error::Other(format!("#error {}", msg.trim()))));
                }
                other => {
                    return Err(at(Error::Other(format!(
                        "TODO: #{other} (not observed in SDK resources)"
                    ))));
                }
            }
            self.out.push('\n');
        }
        self.flush(&mut block)?;
        if !stack.is_empty() {
            return Err(Error::Other(format!(
                "{}: unterminated #if",
                path.display()
            )));
        }
        Ok(())
    }

    fn flush(&mut self, block: &mut Vec<CppToken>) -> Result<()> {
        if block.is_empty() {
            return Ok(());
        }
        // Keep line count: a macro call spanning lines loses its newlines, add them back
        // after the expansion.
        let lines = block
            .iter()
            .filter(|t| matches!(t, CppToken::Newline))
            .count();
        let expanded = self.macros.expand(block)?;
        let emitted = expanded
            .iter()
            .filter(|t| matches!(t, CppToken::Newline))
            .count();
        for t in &expanded {
            self.out.push_str(t.text());
        }
        for _ in emitted..lines {
            self.out.push('\n');
        }
        block.clear();
        Ok(())
    }

    fn condition(&self, kind: &str, rest: &[CppToken]) -> Result<bool> {
        match kind {
            "ifdef" | "ifndef" => {
                let Some(CppToken::Ident(n)) = rest.iter().find(|t| !t.is_space()) else {
                    return Err(Error::Other(format!("#{kind} without a name")));
                };
                Ok(self.macros.is_defined(n) == (kind == "ifdef"))
            }
            _ => CppCondition::eval(rest, &self.macros),
        }
    }

    /// `"file"`: the including file's directory, then `-I`; `<file>`: `-I` only. Names
    /// use `\` and the wrong case (SDK headers), so each component matches
    /// case-insensitively.
    fn include_target(&self, from: &Path, rest: &[CppToken]) -> Result<PathBuf> {
        let text: String = rest.iter().map(CppToken::text).collect();
        let text = text.trim();
        let (name, quoted) = if let Some(q) = text.strip_prefix('"') {
            (q.split('"').next().unwrap_or(""), true)
        } else if let Some(a) = text.strip_prefix('<') {
            (a.split('>').next().unwrap_or(""), false)
        } else {
            return Err(Error::Other(format!("TODO: computed #include {text}")));
        };
        let name = name.replace('\\', "/");
        let mut dirs = Vec::new();
        if quoted && let Some(dir) = from.parent() {
            dirs.push(dir.to_path_buf());
        }
        dirs.extend(self.include_dirs.iter().cloned());
        for dir in &dirs {
            if let Some(found) = Self::find(dir, &name) {
                return Ok(found);
            }
        }
        Err(Error::Other(format!("#include {name}: not found")))
    }

    fn find(dir: &Path, name: &str) -> Option<PathBuf> {
        let mut at = dir.to_path_buf();
        for part in Path::new(name).components() {
            match part {
                Component::Normal(p) => {
                    let want = p.to_str()?;
                    let exact = at.join(want);
                    at = if exact.exists() {
                        exact
                    } else {
                        std::fs::read_dir(&at)
                            .ok()?
                            .flatten()
                            .map(|e| e.path())
                            .find(|p| {
                                p.file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|n| n.eq_ignore_ascii_case(want))
                            })?
                    };
                }
                Component::ParentDir => at.push(".."),
                _ => {}
            }
        }
        at.is_file().then_some(at)
    }
}

#[cfg(test)]
mod tests;
