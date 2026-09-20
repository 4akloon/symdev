//! The attribute id table and the value layout each id carries (spec §4.9).

/// How the bytes after the 16-bit id are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    /// One number; `width` and `height` add a unit byte on `<svg>` (§4.5).
    Number,
    /// A number clamped to 0…1.
    Opacity,
    /// A bare 4-byte colour word.
    Colour,
    /// A flag byte, then a colour word or a gradient reference (§4.6).
    Fill,
    /// Length byte + UTF-16LE.
    Text,
    /// A one-byte count, then that many strings.
    TextList,
    /// A one-byte count, then that many numbers.
    NumberList,
    /// Four numbers, no count.
    ViewBox,
    /// Command and value counts, then the numbers (§4.10).
    PathData,
    /// Six numbers and a kind word (§4.11).
    Transform,
    /// A 4-byte little-endian keyword code.
    Enum32,
    /// A single keyword byte.
    Enum8,
    /// Two bytes on `<svg>` and only for `none`; a string on `<image>`.
    PreserveAspectRatio,
    /// A string, followed on `<use>` by a second string naming the target.
    Href,
}

/// Name → id and layout, for the elements an application icon is built from.
/// Ids the survey could not pin down are deliberately absent so that using one
/// is an error rather than a guess.
pub fn lookup(name: &str) -> Option<(u16, Layout)> {
    use Layout::*;
    Some(match name {
        "fill" | "solid-color" => (0x0000, Fill),
        "stroke" => (0x0001, Colour),
        "stroke-width" => (0x0002, Number),
        "visibility" => (0x0003, Enum32),
        "font-family" => (0x0004, Text),
        "font-size" => (0x0005, Number),
        "font-style" => (0x0006, Enum32),
        "font-weight" => (0x0007, Enum32),
        "stroke-dasharray" => (0x0008, NumberList),
        "display" => (0x0009, Enum32),
        "fill-rule" => (0x000a, Text),
        "stroke-linecap" => (0x000b, Text),
        "stroke-linejoin" => (0x000c, Text),
        "stroke-dashoffset" => (0x000d, Number),
        "stroke-miterlimit" => (0x000e, Number),
        "color" => (0x000f, Colour),
        "text-anchor" => (0x0010, Enum32),
        "fill-opacity" | "solid-opacity" => (0x0016, Opacity),
        "stroke-opacity" => (0x0017, Opacity),
        "opacity" => (0x0018, Opacity),
        "width" => (0x001a, Number),
        "height" => (0x001b, Number),
        "r" => (0x001c, Number),
        "rx" => (0x001d, Number),
        "ry" => (0x001e, Number),
        "cx" => (0x002e, Number),
        "cy" => (0x002f, Number),
        "y" => (0x0030, Number),
        "x" => (0x0031, Number),
        "y1" => (0x0032, Number),
        "y2" => (0x0033, Number),
        "x1" => (0x0034, Number),
        "x2" => (0x0035, Number),
        "transform" => (0x0042, Transform),
        "version" => (0x004c, Number),
        "points" => (0x004e, PathData),
        "d" => (0x004f, PathData),
        "stop-color" => (0x0051, Colour),
        "fx" => (0x0052, Number),
        "fy" => (0x0053, Number),
        "offset" => (0x0054, Number),
        "spreadMethod" | "gradientUnits" | "zoomAndPan" => (spread_id(name), Enum8),
        "stop-opacity" => (0x0057, Number),
        "viewBox" => (0x0058, ViewBox),
        "baseProfile" => (0x0059, Text),
        "preserveAspectRatio" => (0x005b, PreserveAspectRatio),
        "id" => (0x005c, Text),
        "xml:base" => (0x005d, Text),
        "xml:lang" => (0x005e, Text),
        "xml:space" => (0x005f, Text),
        "requiredExtensions" => (0x0060, TextList),
        "requiredFeatures" => (0x0061, TextList),
        "systemLanguage" => (0x0062, TextList),
        "xlink:href" => (0x006d, Href),
        _ => return None,
    })
}

fn spread_id(name: &str) -> u16 {
    match name {
        "spreadMethod" => 0x0055,
        "gradientUnits" => 0x0056,
        _ => 0x005a,
    }
}

/// Attributes the tool recognises and writes no bytes for (spec §4.9), plus
/// `enable-background` and `overflow`, both confirmed dropped by experiment 59.
/// Namespace declarations are handled by the caller.
pub fn dropped(name: &str) -> bool {
    matches!(
        name,
        "class"
            | "pathLength"
            | "color-rendering"
            | "color-interpolation"
            | "letter-spacing"
            | "word-spacing"
            | "enable-background"
            | "overflow"
            | "externalResourcesRequired"
            | "gradientTransform"
            | "focusable"
            | "initialVisibility"
            | "target"
            | "media"
    ) || name.starts_with("nav-")
}

/// The properties the tool's `style` parser turns into attribute records
/// (experiment 59). `stop-color`, `stop-opacity`, `d`, `points` and
/// `transform` are excluded on purpose: the parser recognises the last three
/// and mangles or crashes on them, and it writes a wrong value for the first
/// two, so `style` carrying one of those is refused instead.
pub fn style_property(name: &str) -> bool {
    matches!(
        name,
        "fill"
            | "stroke"
            | "stroke-width"
            | "visibility"
            | "font-family"
            | "font-size"
            | "font-style"
            | "font-weight"
            | "stroke-dasharray"
            | "display"
            | "fill-rule"
            | "stroke-linecap"
            | "stroke-linejoin"
            | "stroke-dashoffset"
            | "stroke-miterlimit"
            | "color"
            | "fill-opacity"
            | "stroke-opacity"
            | "opacity"
            | "text-anchor"
    )
}

/// The keyword codes of the `Enum32` and `Enum8` attributes (spec §4.9).
pub fn keyword(attribute: &str, value: &str) -> Option<u32> {
    Some(match (attribute, value) {
        ("visibility", "visible" | "inherit") => 0,
        ("visibility", "hidden") => 1,
        ("visibility", "collapse") => 3,
        ("display", "inline" | "block" | "inherit") => 0,
        ("display", "none") => 16,
        ("font-style", "normal") => 0,
        ("font-style", "italic") => 1,
        ("font-style", "oblique") => 2,
        ("font-weight", "normal") => 0,
        ("font-weight", "bold") => 1,
        ("font-weight", "bolder") => 2,
        ("font-weight", "lighter") => 3,
        ("font-weight", w) if w.len() == 3 && w.ends_with("00") => match w.as_bytes()[0] {
            digit @ b'1'..=b'9' => 4 + u32::from(digit - b'1'),
            _ => return None,
        },
        ("spreadMethod", "pad") => 0,
        ("spreadMethod", "reflect") => 1,
        ("spreadMethod", "repeat") => 2,
        ("gradientUnits", "userSpaceOnUse") => 0,
        ("gradientUnits", "objectBoundingBox") => 1,
        // Only `middle` was observed (experiment 59); the other keywords'
        // codes are still unknown, so they are an error.
        ("text-anchor", "middle") => 1,
        ("zoomAndPan", "disable") => 0,
        _ => return None,
    })
}
