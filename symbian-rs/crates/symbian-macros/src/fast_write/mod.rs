//! `write_pieces!`, the proc macro behind `symbian_fmt::write!` and `writeln!`.
//!
//! It is called by those two `macro_rules!` wrappers, never by hand, with
//! `$crate ; <newline: true|false> ; $dst ; $fmt ; $($arg),*`. `macro_rules!` has
//! already split the invocation into expressions — which a token scan cannot do
//! reliably, `f::<A, B>(x)` has a comma at the top level — so this only has to read
//! the format string and the named-argument form `name = value`.
//!
//! Whatever it does not take apart goes to `::core::write!` (or `writeln!`) with the
//! tokens it was given, so that invocation is `write!` in every respect, rustc's
//! diagnostics included.

use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, TokenStream, TokenTree};

mod expand;
mod literal;
mod plan;
mod template;

#[cfg(test)]
mod tests;

use plan::{Explicit, Plan, Source};

/// The invocation, split at its semicolons and its argument commas.
struct Invocation {
    krate: TokenStream,
    newline: bool,
    dst: TokenStream,
    format: TokenTree,
    args: Vec<TokenStream>,
}

pub fn write_pieces(input: TokenStream) -> TokenStream {
    let Some(call) = Invocation::split(input) else {
        return crate::tokens("::core::compile_error!(\"symbian_fmt::write_pieces! is called by write! only\");");
    };
    match call.plan() {
        Some((plan, values)) => call.fast(&plan, values),
        None => call.core(),
    }
}

impl Invocation {
    fn split(input: TokenStream) -> Option<Self> {
        let mut parts = vec![TokenStream::new()];
        for tree in input {
            match &tree {
                TokenTree::Punct(p) if p.as_char() == ';' => parts.push(TokenStream::new()),
                _ => parts.last_mut()?.extend([tree]),
            }
        }
        let [krate, newline, dst, format, args] = <[TokenStream; 5]>::try_from(parts).ok()?;
        let mut args_list = vec![TokenStream::new()];
        for tree in args {
            match &tree {
                TokenTree::Punct(p) if p.as_char() == ',' => args_list.push(TokenStream::new()),
                _ => args_list.last_mut()?.extend([unwrap_none(tree)]),
            }
        }
        args_list.retain(|arg| !arg.is_empty());
        let format = unwrap_none(format.into_iter().next()?);
        Some(Self {
            krate,
            newline: newline.to_string() == "true",
            dst,
            format,
            args: args_list.into_iter().map(flatten).collect(),
        })
    }

    /// The plan and, per slot, the tokens of its value; `None` for `core::write!`.
    fn plan(&self) -> Option<(Plan, Vec<TokenStream>)> {
        let TokenTree::Literal(format) = &self.format else { return None };
        let value = literal::string_value(&format.to_string())?;
        let segments = template::segments(&value)?;
        let explicit: Vec<(Explicit, TokenStream)> = self.args.iter().map(explicit).collect();
        let specs: Vec<Explicit> = explicit.iter().map(|(spec, _)| spec.clone()).collect();
        let plan = Plan::new(&segments, &specs, self.newline)?;
        let values = plan
            .slots
            .iter()
            .map(|slot| match slot {
                Source::Explicit(index) => borrow(explicit[*index].1.clone()),
                // A capture is an identifier with the format string's span, which is
                // what makes it resolve in the caller's scope — as `format_args!` does.
                Source::Capture(name) => borrow(TokenTree::Ident(Ident::new(name, format.span())).into()),
            })
            .collect();
        Some((plan, values))
    }

    fn fast(&self, plan: &Plan, values: Vec<TokenStream>) -> TokenStream {
        let text = expand::expansion(plan);
        fill(crate::tokens(&text), &|hole: &str| match hole {
            "krate" => Some(self.krate.clone()),
            "dst" => Some(self.dst.clone()),
            _ => values.get(hole.strip_prefix("slot")?.parse::<usize>().ok()?).cloned(),
        })
    }

    /// `::core::write!(dst, fmt, args…)`, from the tokens as they came.
    fn core(&self) -> TokenStream {
        let name = if self.newline { "writeln" } else { "write" };
        let mut inner = self.dst.clone();
        let comma = || TokenTree::Punct(Punct::new(',', Spacing::Alone));
        inner.extend([comma(), self.format.clone()]);
        for arg in &self.args {
            inner.extend([comma()]);
            inner.extend(arg.clone());
        }
        let mut out = crate::tokens(&format!("::core::{name}!"));
        out.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, inner))]);
        out
    }
}

/// An argument's plan entry and the tokens of its value.
fn explicit(arg: &TokenStream) -> (Explicit, TokenStream) {
    let trees: Vec<TokenTree> = arg.clone().into_iter().collect();
    let named = match trees.as_slice() {
        [TokenTree::Ident(name), TokenTree::Punct(eq), rest @ ..]
            if eq.as_char() == '='
                && !rest.is_empty()
                && !matches!(rest.first(), Some(TokenTree::Punct(p)) if p.as_char() == '=') =>
        {
            Some((name.to_string(), rest.iter().cloned().collect::<TokenStream>()))
        }
        _ => None,
    };
    let (name, value) = match named {
        Some((name, value)) => (Some(name), value),
        None => (None, arg.clone()),
    };
    let folded = folded(&value);
    (Explicit { name, folded }, value)
}

/// What rustc folds into the template for this argument, if it is a literal it folds.
fn folded(value: &TokenStream) -> Option<String> {
    let mut trees = value.clone().into_iter();
    let (only, None) = (trees.next()?, trees.next()) else { return None };
    match only {
        TokenTree::Group(group)
            if matches!(group.delimiter(), Delimiter::Parenthesis | Delimiter::None) =>
        {
            folded(&group.stream())
        }
        TokenTree::Literal(lit) => {
            let source = lit.to_string();
            literal::string_value(&source).or_else(|| literal::folded_integer(&source))
        }
        _ => None,
    }
}

/// `&(value)`.
fn borrow(value: TokenStream) -> TokenStream {
    let mut out: TokenStream = TokenTree::Punct(Punct::new('&', Spacing::Alone)).into();
    out.extend([TokenTree::Group(Group::new(Delimiter::Parenthesis, value))]);
    out
}

/// The tokens inside a `$x:expr` fragment's invisible group.
fn unwrap_none(tree: TokenTree) -> TokenTree {
    match tree {
        TokenTree::Group(g) if g.delimiter() == Delimiter::None => {
            let mut inner = g.stream().into_iter();
            match (inner.next(), inner.next()) {
                (Some(only), None) => unwrap_none(only),
                _ => TokenTree::Group(g),
            }
        }
        other => other,
    }
}

/// The tokens of an argument without the invisible group around the whole of it, so
/// `name = value` reads as a named argument and not as an assignment expression.
fn flatten(arg: TokenStream) -> TokenStream {
    let mut trees = arg.clone().into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Group(g)), None) if g.delimiter() == Delimiter::None => g.stream(),
        _ => arg,
    }
}

/// `tokens` with every `@name` replaced by `hole(name)`.
fn fill(tokens: TokenStream, hole: &dyn Fn(&str) -> Option<TokenStream>) -> TokenStream {
    let mut out = TokenStream::new();
    let mut trees = tokens.into_iter().peekable();
    while let Some(tree) = trees.next() {
        match tree {
            TokenTree::Punct(p) if p.as_char() == '@' => {
                if let Some(TokenTree::Ident(name)) = trees.peek()
                    && let Some(with) = hole(&name.to_string())
                {
                    trees.next();
                    out.extend(with);
                    continue;
                }
                out.extend([TokenTree::Punct(p)]);
            }
            TokenTree::Group(g) => {
                let mut filled = Group::new(g.delimiter(), fill(g.stream(), hole));
                filled.set_span(g.span());
                out.extend([TokenTree::Group(filled)]);
            }
            other => out.extend([other]),
        }
    }
    out
}
