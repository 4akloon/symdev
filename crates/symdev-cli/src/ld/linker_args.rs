//! `LinkerArgs`: rustc's argv to cargo's linker, as experiment 114 §1.1 recorded it.
use std::ffi::OsString;
use std::path::PathBuf;

use symdev_core::{Error, Result};

/// The inputs, in rustc's order, and the `-o` path. Every other argument rustc was seen
/// to pass is either already on 0.3.0's line (`--gc-sections`, `--strip-debug`) or means
/// nothing to it (`--as-needed`, `-Bstatic`, `-Bdynamic`, `-z noexecstack`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkerArgs {
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    /// rustc's `-L <tmp>/raw-dylibs`: empty for every program observed. `LinkRun`
    /// refuses a non-empty one, which would mean a `raw-dylib` import nobody has seen.
    pub raw_dylibs: Vec<PathBuf>,
}

impl LinkerArgs {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut words = Vec::new();
        for a in args {
            words.push(a.into_string().map_err(|a| {
                Error::Other(format!(
                    "symdev-ld: an argument is not UTF-8: {}",
                    a.to_string_lossy()
                ))
            })?);
        }
        let mut rest = words.as_slice();
        if let [flavor, gnu, tail @ ..] = rest
            && flavor == "-flavor"
            && gnu == "gnu"
        {
            rest = tail;
        }
        let (mut inputs, mut output, mut raw_dylibs) = (Vec::new(), None, Vec::new());
        let mut it = rest.iter();
        while let Some(word) = it.next() {
            match word.as_str() {
                "--as-needed" | "-Bstatic" | "-Bdynamic" | "--gc-sections" | "--strip-debug" => {}
                "-z" => match it.next().map(String::as_str) {
                    Some("noexecstack") => {}
                    other => return Err(unseen(&format!("-z {}", other.unwrap_or("")))),
                },
                "-L" => raw_dylibs.push(PathBuf::from(value(&mut it, "-L")?)),
                "-o" if output.is_none() => output = Some(PathBuf::from(value(&mut it, "-o")?)),
                w if !w.starts_with('-') && (w.ends_with(".o") || w.ends_with(".rlib")) => {
                    inputs.push(PathBuf::from(w))
                }
                w => return Err(unseen(w)),
            }
        }
        let output = output.ok_or_else(|| {
            Error::Other("symdev-ld: rustc passed no `-o`; symdev-ld is cargo's linker".into())
        })?;
        if inputs.is_empty() {
            return Err(Error::Other(
                "symdev-ld: rustc passed no object or rlib".into(),
            ));
        }
        Ok(Self {
            inputs,
            output,
            raw_dylibs,
        })
    }
}

fn value<'a>(it: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<&'a String> {
    it.next()
        .ok_or_else(|| Error::Other(format!("symdev-ld: `{flag}` without a value")))
}

fn unseen(word: &str) -> Error {
    Error::Other(format!(
        "symdev-ld: rustc passed `{word}`, which experiment 114 never saw it pass for \
         arm-symbian-e32; TODO: support it once observed (not observed). Report it with the \
         command that produced it."
    ))
}
