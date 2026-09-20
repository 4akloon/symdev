//! `Entry`: what `#[symbian_std::main]` understood, and the wrapper it writes.

use crate::signature::Signature;

/// The path an application writes, and the path the generated code names things by.
pub const ATTRIBUTE: &str = "symbian_std::main";

/// The C++-mangled `E32Main()` that `eexe.lib`'s startup calls (experiment 65a). The
/// link line names it with `-u _Z7E32Mainv` so the archive member defining it is
/// pulled; that flag is `RustBuild`'s, and this is the definition it looks for.
pub const E32MAIN: &str = "_Z7E32Mainv";

/// Which of the two process shapes the application is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A console application: `main` runs to completion and its value is the exit code.
    Console,
    /// An Avkon application, reserved for step 75. It cannot be the console shape,
    /// because CONE creates and runs the `CCoeScheduler` itself and `CCoeEnv` is a
    /// `CActive` on it — a second `CActiveScheduler` would panic the thread.
    Gui,
}

impl Shape {
    /// The attribute's whole grammar: nothing, or one word naming the shape. It is
    /// written out here, rather than only where a shape is generated, so that adding
    /// the GUI entry in step 75 changes what `gui` *does* and not what it *is*.
    fn parse(arguments: &str) -> Result<Self, String> {
        match arguments.trim() {
            "" => Ok(Self::Console),
            "gui" => Ok(Self::Gui),
            other => Err(format!(
                "`#[{ATTRIBUTE}]` takes no arguments, or `gui` for an Avkon application \
                 (step 75); found `{other}`"
            )),
        }
    }
}

/// The application's entry point: the shape it asked for and the function it names.
pub struct Entry {
    shape: Shape,
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
        if shape == Shape::Gui {
            return Err(format!(
                "`#[{ATTRIBUTE}(gui)]` is not implemented yet: an Avkon application's entry \
                 point must not create a `CActiveScheduler`, because CONE creates and runs \
                 `CCoeScheduler` itself and `CCoeEnv` is a `CActive` on it (step 75). Write \
                 `#[{ATTRIBUTE}]` for a console application."
            ));
        }
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
        Ok(Self { shape })
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

    /// The `E32Main()` the loader calls: it runs `main` and hands the value to
    /// `IntoExitCode`, which is the one place a Rust value becomes a `TInt`.
    pub fn wrapper(&self) -> String {
        match self.shape {
            Shape::Console => format!(
                "#[unsafe(export_name = \"{E32MAIN}\")]\n\
                 pub extern \"C\" fn __symbian_e32main() -> i32 {{\n    \
                 ::symbian_std::__rt::ExitCode::from_main({}())\n\
                 }}\n",
                Self::NAME
            ),
            // Unreachable: `parse` refuses `gui` until step 75 says what it generates —
            // a `CAknApplication` handed to `EikStart::RunApplication`, with no active
            // scheduler of our own. This arm is where that code will go.
            Shape::Gui => String::new(),
        }
    }
}
