//! The `.mmp` directive split of mmp-frontend-spec.md §5.5/§5.6, and the number format
//! of §11.1.
use crate::bld::ParseError;

/// Parsed, recorded, and of no effect on this build path (§5.5). The SDK reads some of
/// them on other platforms and ignores the rest; either way nothing here acts on them,
/// so the front end says so once instead of dropping them silently.
pub const IGNORED: &[&str] = &[
    "ALWAYS_BUILD_AS_ARM",
    "ARMFPU",
    "BYTEPAIRCOMPRESSTARGET",
    "COMPRESSTARGET",
    "DEBUGGABLE",
    "DEBUGGABLE_UDEBONLY",
    "DEBUGLIBRARY",
    "DOCUMENT",
    "EPOCDATALINKADDRESS",
    "EPOCFIXEDPROCESS",
    "EPOCHEAPSIZE",
    "EPOCPROCESSPRIORITY",
    "EPOCSTACKSIZE",
    "EPOCCALLDLLENTRYPOINTS",
    "EXPORTLIBRARY",
    "EXPORTUNFROZEN",
    "FIRSTLIB",
    "INFLATECOMPRESSTARGET",
    "LINKAS",
    "LINKEROPTION",
    "NOCOMPRESSTARGET",
    "NOEXPORTLIBRARY",
    "OPTION_REPLACE",
    "PAGED",
    "SMPSAFE",
    "SRCDBG",
    "STRICTDEPEND",
    "UNPAGED",
    "VERSION",
    "WCHARENTRYPOINT",
];

/// Directives that change what the image or the package is and that this path cannot
/// honour: dropping one quietly would ship the wrong binary (§5.6).
pub fn rejected(name: &str) -> Option<&'static str> {
    Some(match name {
        "ASSPABI" | "ASSPEXPORTS" | "ASSPLIBRARY" => {
            "ASSP linking is fatal on a generic platform, and GCCE is generic"
        }
        "AIF" => "the pre-9.x application information file; use a registration resource",
        "SYSTEMRESOURCE" => "z\\system\\data\\ resources are not installable on a secure platform",
        "RESOURCE" => "the flat resource form; write START RESOURCE … END",
        "RAMTARGET" | "ROMTARGET" => "extra release copies (not observed)",
        "FEATUREVARIANT" => "feature-variant builds (not observed)",
        _ => return None,
    })
}

/// `EPOCSTACKSIZE`, `UID`, `SECUREID`, `VENDORID` and friends: lower-cased, then either
/// 1–10 decimal digits or `0x` and 1–8 hexadecimal digits (§11.1).
pub fn number(token: &str) -> Result<u32, ParseError> {
    let lower = token.to_ascii_lowercase();
    let bad = || ParseError(format!("not a number: {token}"));
    match lower.strip_prefix("0x") {
        Some(hex) => {
            if hex.is_empty() || hex.len() > 8 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(bad());
            }
            u32::from_str_radix(hex, 16).map_err(|_| bad())
        }
        None => {
            if lower.is_empty() || lower.len() > 10 || !lower.chars().all(|c| c.is_ascii_digit()) {
                return Err(bad());
            }
            // Decimal is capped at ten characters, not at the u32 range (§11.1).
            Ok(lower.parse::<u64>().map_err(|_| bad())? as u32)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn hexadecimal_is_capped_at_eight_digits_and_decimal_at_ten_characters() {
        assert_eq!(number("0xE7351C20").unwrap(), 0xe735_1c20);
        assert_eq!(number("100").unwrap(), 100);
        assert!(number("0x100000000").is_err());
        assert!(number("12345678901").is_err());
        assert!(number("0x").is_err());
        assert!(number("12a").is_err());
        // Ten decimal digits parse and wrap, as the SDK's format does.
        assert_eq!(number("9999999999").unwrap(), 9_999_999_999u64 as u32);
    }
}
