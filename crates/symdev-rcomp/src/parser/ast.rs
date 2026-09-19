//! Syntax tree of resource source: statements, members, values, expressions.

/// Width of a length prefix (`STRUCT X BYTE`, `LEN BYTE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RssWidth {
    Byte,
    Word,
}

/// How a text member stores its length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RssTextForm {
    /// `LTEXT`: leading length (characters) as a byte.
    Counted,
    /// `BUF`: no length; runs to the end of the resource.
    Bare,
    /// `TEXT`: zero-terminated.
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RssType {
    Byte,
    Word,
    Long,
    Double,
    /// `bits`: 16 (`LTEXT`, `BUF`, `TEXT` under `rcomp -u`, and the `16` forms) or 8.
    Text {
        form: RssTextForm,
        bits: u8,
    },
    Link,
    Llink,
    Srlink,
    Struct,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RssExpr {
    Int(i64),
    Real(f64),
    Char(u32),
    Name(String),
    Neg(Box<RssExpr>),
    Not(Box<RssExpr>),
    Binary(char, Box<RssExpr>, Box<RssExpr>),
}

/// One piece of a text value: `"literal"`, `<char code>`, or a `rls_string` name.
#[derive(Debug, Clone, PartialEq)]
pub enum RssTextPart {
    Literal(Vec<u32>),
    Code(RssExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RssValue {
    /// A number, a name (enum, `rls_*`, resource), or an expression.
    Expr(RssExpr),
    Text(Vec<RssTextPart>),
    List(Vec<RssValue>),
    Struct(RssStructValue),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RssStructValue {
    pub struct_name: String,
    pub fields: Vec<(String, RssValue)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RssMember {
    pub name: String,
    pub ty: RssType,
    /// `BUF<n>` / `LTEXT name(n)`: maximum length.
    pub max_len: Option<RssExpr>,
    /// `None`: scalar. `Some(None)`: `name[]`. `Some(Some(n))`: `name[n]`.
    pub array: Option<Option<RssExpr>>,
    /// `LEN BYTE` / `LEN WORD` before an array.
    pub len_prefix: Option<RssWidth>,
    pub default: Option<RssValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RssStruct {
    pub name: String,
    /// `STRUCT X BYTE { … }`: embedded instances carry their length.
    pub len_prefix: Option<RssWidth>,
    pub members: Vec<RssMember>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RssResource {
    pub value: RssStructValue,
    pub name: Option<String>,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RssItem {
    Name(String),
    Uid2(RssExpr),
    Uid3(RssExpr),
    CharacterSet(String),
    Struct(RssStruct),
    Resource(RssResource),
    Enum(Vec<(String, Option<RssExpr>)>),
    /// `rls_string NAME "text"` and the other `rls_*` forms: a named constant.
    Rls(String, RssValue),
}
