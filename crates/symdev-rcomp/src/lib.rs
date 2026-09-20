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

pub use compiler::RscCompiled;
pub use compiler::RscResourceData;
pub use cpp::RssPreprocessor;
pub use rcomp::Rcomp;
pub use resource::{Rsc, RscAppRegistration, RscLtext16};
pub use uid::RscUid;
