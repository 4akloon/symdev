//! Plain base64 decoding for PEM bodies (no padding/newline requirements assumed).
use symdev_core::{Error, Result};

pub(super) fn base64_decode(s: &str) -> Result<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'=' {
            break;
        }
        let a = val(bytes[i]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        let b = if i + 1 < bytes.len() && bytes[i + 1] != b'=' {
            val(bytes[i + 1]).ok_or_else(|| Error::Other("invalid base64".into()))?
        } else {
            break;
        };
        out.push((a << 2) | (b >> 4));
        if i + 2 >= bytes.len() || bytes[i + 2] == b'=' {
            break;
        }
        let c = val(bytes[i + 2]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        out.push((b << 4) | (c >> 2));
        if i + 3 >= bytes.len() || bytes[i + 3] == b'=' {
            break;
        }
        let d = val(bytes[i + 3]).ok_or_else(|| Error::Other("invalid base64".into()))?;
        out.push((c << 6) | d);
        i += 4;
    }
    Ok(out)
}
