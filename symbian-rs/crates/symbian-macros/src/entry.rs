//! `Entry`: what `#[symbian_std::main]` understood, and the wrapper it writes.

use crate::signature::Signature;

/// The path an application writes, and the path the generated code names things by.
pub const ATTRIBUTE: &str = "symbian_std::main";

/// The C++-mangled `E32Main()` that `eexe.lib`'s startup calls (experiment 65a). The
/// link line names it with `-u _Z7E32Mainv` so the archive member defining it is
/// pulled; that flag is `RustBuild`'s, and this is the definition it looks for.
pub const E32MAIN: &str = "_Z7E32Mainv";

/// The path the generated GUI entry names the UI crate by. It is reached through
/// `symbian-std` so that an application depends on one crate, exactly as the console
/// shape reaches `ExitCode` through `::symbian_std`.
pub const UI: &str = "::symbian_std::ui";

/// Which of the two process shapes the application is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A console application: `main` runs to completion and its value is the exit code.
    Console,
    /// An Avkon application (step 75). It cannot be the console shape, because CONE
    /// creates and runs the `CCoeScheduler` itself and `CCoeEnv` is a `CActive` on it —
    /// a second `CActiveScheduler` would panic the thread. So this shape writes **no**
    /// `E32Main` at all: for a GUI application the C++ shim owns the entry point and
    /// hands the process to `EikStart::RunApplication`, and what Rust exports instead
    /// are the `symrs_app_*` functions that shim imports.
    Gui,
}

impl Shape {
    /// The attribute's whole grammar: nothing, or one word naming the shape.
    fn parse(arguments: &str) -> Result<Self, String> {
        match arguments.trim() {
            "" => Ok(Self::Console),
            "gui" => Ok(Self::Gui),
            other => Err(format!(
                "`#[{ATTRIBUTE}]` takes no arguments, or `gui` for an Avkon application; \
                 found `{other}`"
            )),
        }
    }
}

/// The application's entry point: the shape it asked for and the function it names.
pub struct Entry {
    shape: Shape,
    /// For [`Shape::Gui`], the application type `fn main` returns.
    app: String,
}

impl Entry {
    /// The name the entry point must have. One process has one entry point, and the
    /// attribute is named after it, so requiring the name keeps a second `#[main]` in
    /// the same crate from becoming a duplicate-symbol error at link time.
    pub const NAME: &'static str = "main";

    /// Reads the attribute's arguments and the function it was applied to, in the
    /// rendered form of their token streams.
    pub fn parse(arguments: &str, item: &str) -> Result<Self, String> {
        let shape = Shape::parse(arguments)?;
        let signature = Signature::parse(item, ATTRIBUTE)?;
        if signature.name != Self::NAME {
            return Err(format!(
                "`#[{ATTRIBUTE}]` names the program's entry point, so the function must be \
                 called `{}` (found `{}`)",
                Self::NAME,
                signature.name
            ));
        }
        if signature.takes_arguments() {
            return Err(format!(
                "`fn {}` must take no arguments: `E32Main()` is called with none, and a \
                 Symbian process reads its command line with `RProcess::CommandLine` \
                 instead (found `{}`)",
                Self::NAME,
                signature.parameters
            ));
        }
        for qualifier in &signature.qualifiers {
            Self::reject_qualifier(qualifier)?;
        }
        let app = match shape {
            Shape::Console => String::new(),
            // The type is read from the signature rather than from the attribute: the
            // application object is what `fn main` returns, and writing the name twice
            // is a way for the two to drift apart.
            Shape::Gui => signature
                .returns
                .ok_or_else(|| {
                    format!(
                        "`#[{ATTRIBUTE}(gui)]`: `fn {}` must return the application type, \
                         which is what the framework builds and calls — an `impl App` \
                         struct, for example `fn main() -> Notes {{ Notes::new() }}`",
                        Self::NAME
                    )
                })?
                .to_string(),
        };
        Ok(Self { shape, app })
    }

    fn reject_qualifier(qualifier: &str) -> Result<(), String> {
        let reason = match qualifier {
            "pub" | "const" => return Ok(()),
            "async" => {
                "an `async fn main` needs an executor, and the active-scheduler bridge is \
                 step 73"
            }
            "unsafe" => "the generated `E32Main` calls it from safe code",
            "extern" => "the generated `E32Main` is the only `extern` function here",
            _ => "the entry point is a plain `fn main`",
        };
        Err(format!(
            "`#[{ATTRIBUTE}]` cannot be applied to a `{qualifier} fn`: {reason}"
        ))
    }

    /// The `E32Main()` the loader calls: it hands `main` to `symbian_std::__start`,
    /// which is the one place a Rust `fn main` becomes a `TInt`.
    ///
    /// The line is the same whether the application is `#![no_std]` or has a real
    /// `std` (design spec §11 step 77). `symbian-std` has one `__start` per shape:
    /// without `std` it converts through `IntoExitCode`, with `std` it is
    /// `std::os::symbian::start`, which is `lang_start` plus the main thread's
    /// thread-local destructors. Writing the choice here instead would make the
    /// attribute have to know which half of the SDK the application built against.
    pub fn wrapper(&self) -> String {
        match self.shape {
            Shape::Console => format!(
                "#[unsafe(export_name = \"{E32MAIN}\")]\n\
                 pub extern \"C\" fn __symbian_e32main() -> i32 {{\n    \
                 ::symbian_std::__start({})\n\
                 }}\n",
                Self::NAME
            ),
            // No `E32Main`, and no active scheduler: the C++ shim owns both, and CONE
            // is already inside `CActiveScheduler::Start()` by the time any of this
            // runs. What Rust exports is the `symrs_app_*` functions the shim forwards
            // each Avkon virtual to, which `symbian-ui`'s `__export_app!` writes next to
            // their bodies; `create` calls `main` so that the application's own
            // `fn main` stays the place its object is built.
            Shape::Gui => format!(
                "{UI}::__export_app!({app}, {main});\n",
                app = self.app,
                main = Self::NAME
            ),
        }
    }
}
