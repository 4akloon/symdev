//! Symbian resource compiler: `.rss` → `.rsc` / `.rsg` natively (preprocessor, parser,
//! compiler, writer), plus the registration-resource writer used when a project has no
//! `_reg.rss`.

mod compiler;
mod cpp;
mod lexer;
mod pack;
mod parser;
mod rcomp;
mod resource;
mod rsc;
mod scsu;
mod uid;

pub use compiler::{RscCompiled, RscCompiledResource, RscResourceData, RscSegment, RssCompiler};
pub use cpp::RssPreprocessor;
pub use lexer::{RssLexer, RssSpanned, RssToken};
pub use parser::RssParser;
pub use rcomp::Rcomp;
pub use resource::{Rsc, RscAppRegistration, RscLtext16, RscResource};
pub use scsu::RscScsu;
pub use uid::RscUid;
