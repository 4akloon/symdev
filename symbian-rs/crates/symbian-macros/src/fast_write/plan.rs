//! What a fast `write!` writes, in order: text known at compile time and arguments.
//!
//! This is where the invocation is held to `format_args!`'s own rules. Anything
//! `format_args!` would reject — an unused argument, `{3}` with two arguments, a
//! positional argument after a named one, a name given twice — is `None`, so the
//! invocation goes to `core::write!` unchanged and rustc reports it in its own words.

use super::template::{Hole, Segment};

/// One explicit argument, as the invocation wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explicit {
    /// `Some` for `name = value`.
    pub name: Option<String>,
    /// The text rustc folds into the template instead of passing the argument, for a
    /// string literal and an integer literal that fits its type
    /// ([`super::literal::folded_integer`]).
    pub folded: Option<String>,
}

/// Where an argument's value comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// The explicit argument at this index.
    Explicit(usize),
    /// A variable of the caller's, named in the string (`{x}`).
    Capture(String),
}

/// One write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    /// The argument in this slot of [`Plan::slots`].
    Arg(usize),
}

/// The writes of one invocation and the values they read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub pieces: Vec<Piece>,
    /// Evaluated once each, in this order — explicit arguments in the order written,
    /// then captures in the order they first appear, which is `format_args!`'s order.
    pub slots: Vec<Source>,
}

impl Plan {
    /// The plan for `segments` over `explicit`, with `"\n"` after it for `writeln!`.
    /// `None` means `core::write!` must handle the invocation.
    pub fn new(segments: &[Segment], explicit: &[Explicit], newline: bool) -> Option<Self> {
        let positional = explicit.iter().take_while(|arg| arg.name.is_none()).count();
        if explicit[positional..].iter().any(|arg| arg.name.is_none()) {
            return None;
        }
        for (i, arg) in explicit.iter().enumerate() {
            if explicit[..i]
                .iter()
                .any(|earlier| earlier.name.is_some() && earlier.name == arg.name)
            {
                return None;
            }
        }
        let mut used = vec![false; explicit.len()];
        let mut captures: Vec<String> = Vec::new();
        let mut next = 0;
        let mut sources = Vec::new();
        for segment in segments {
            let hole = match segment {
                Segment::Text(text) => {
                    sources.push(Err(text.clone()));
                    continue;
                }
                Segment::Hole(hole) => hole,
            };
            let source = match hole {
                Hole::Next => {
                    next += 1;
                    Source::Explicit(next - 1)
                }
                Hole::Index(index) => Source::Explicit(*index),
                Hole::Name(name) => match explicit
                    .iter()
                    .position(|arg| arg.name.as_ref() == Some(name))
                {
                    Some(index) => Source::Explicit(index),
                    None => {
                        if !captures.contains(name) {
                            captures.push(name.clone());
                        }
                        Source::Capture(name.clone())
                    }
                },
            };
            if let Source::Explicit(index) = source {
                *used.get_mut(index)? = true;
                if let Some(text) = &explicit[index].folded {
                    sources.push(Err(text.clone()));
                    continue;
                }
            }
            sources.push(Ok(source));
        }
        if used.contains(&false) {
            return None;
        }
        if newline {
            sources.push(Err(String::from("\n")));
        }
        let plan = Self::from_sources(sources, &captures);
        // Nothing to write at all (`write!(w, "{}", "")`) is still one `write_str("")`
        // in `core::write!`; leave that case to it.
        (!plan.pieces.is_empty()).then_some(plan)
    }

    /// Text runs merged, empty text dropped, and every argument given a slot.
    fn from_sources(sources: Vec<Result<Source, String>>, captures: &[String]) -> Self {
        let mut slots: Vec<Source> = Vec::new();
        let mut pieces = Vec::new();
        let mut text = String::new();
        for source in sources {
            match source {
                Err(more) => text.push_str(&more),
                Ok(source) => {
                    if !text.is_empty() {
                        pieces.push(Piece::Text(std::mem::take(&mut text)));
                    }
                    let slot = slots.iter().position(|s| *s == source).unwrap_or_else(|| {
                        slots.push(source);
                        slots.len() - 1
                    });
                    pieces.push(Piece::Arg(slot));
                }
            }
        }
        if !text.is_empty() {
            pieces.push(Piece::Text(text));
        }
        // Evaluation order: explicit arguments as written, then captures.
        let explicit_first = |s: &Source| match s {
            Source::Explicit(index) => (0, *index),
            Source::Capture(name) => (1, captures.iter().position(|c| c == name).unwrap_or(0)),
        };
        let mut order: Vec<usize> = (0..slots.len()).collect();
        order.sort_by_key(|&i| explicit_first(&slots[i]));
        let remap: Vec<usize> = (0..slots.len())
            .map(|old| order.iter().position(|&o| o == old).unwrap_or(old))
            .collect();
        let pieces = pieces
            .into_iter()
            .map(|p| match p {
                Piece::Arg(slot) => Piece::Arg(remap[slot]),
                text => text,
            })
            .collect();
        let slots = order.into_iter().map(|i| slots[i].clone()).collect();
        Self { pieces, slots }
    }
}
