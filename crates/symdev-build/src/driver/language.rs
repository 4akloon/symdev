//! `SourceLanguage`: the dialect one `SOURCE` entry is compiled in.
use std::path::Path;

use symdev_core::{Error, Result};

/// Which front end a `SOURCE` file goes through. The SDK drives every dialect through the
/// one compiler (`epoc32/tools/compilation_config/gcce.mk`: `CC=arm-none-symbianelf-g++`)
/// and selects the front end by extension: `CPP_LANG_OPTION=-x c++` for `.cpp`,
/// `C_LANG_OPTION=-x c` for `.c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    Cpp,
    C,
}

impl SourceLanguage {
    /// `.cpp` → C++, `.c` → C. Anything else is an error: the SDK's config names only
    /// one more entry (`.cia`, `CIA_LANG_OPTION=-x c++ -S -Wa,-adln`, which needs an
    /// assemble step symdev has never observed), and nothing at all for `.s`/`.S`.
    pub fn of(source: &Path) -> Result<Self> {
        match source.extension().and_then(|e| e.to_str()) {
            Some("cpp") => Ok(Self::Cpp),
            Some("c") => Ok(Self::C),
            _ => Err(Error::Other(format!(
                "TODO: SOURCE {}: only .cpp and .c are compiled; \
                 the assemble/CIA pipeline is not observed",
                source.display()
            ))),
        }
    }

    /// Flags that select and tune the front end, inserted after the machine options.
    ///
    /// C++ keeps the two leniency flags the GCC-12-era SDK headers need (experiment 51);
    /// both are C++-only (`cc1: warning: command-line option '-fpermissive' is valid for
    /// C++/ObjC++ but not for C`, and narrowing is a C++ diagnostic), so the C front end
    /// gets `-x c` instead. `-x c++` is not passed for C++ because the recorded
    /// experiment-5 argv does not carry it and the extension already selects it.
    pub fn args(self) -> Vec<String> {
        match self {
            Self::Cpp => vec!["-fpermissive".into(), "-Wno-narrowing".into()],
            Self::C => vec!["-x".into(), "c".into()],
        }
    }
}
