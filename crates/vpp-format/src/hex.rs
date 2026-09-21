//! Lowercase hexadecimal, as used in key files and resource file names.

const DIGITS: &[u8; 16] = b"0123456789abcdef";

/// Encodes bytes as lowercase hex.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(DIGITS[usize::from(b >> 4)] as char);
        out.push(DIGITS[usize::from(b & 0x0f)] as char);
    }
    out
}

/// Decodes exactly `N` bytes of hex, in either case. `None` if the length or a digit is wrong.
pub(crate) fn decode<const N: usize>(text: &str) -> Option<[u8; N]> {
    let text = text.as_bytes();
    if text.len() != N * 2 {
        return None;
    }
    let mut out = [0; N];
    for (byte, pair) in out.iter_mut().zip(text.chunks_exact(2)) {
        *byte = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Some(out)
}

fn nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let bytes = [0x00, 0x7f, 0x80, 0xff];
        assert_eq!(encode(&bytes), "007f80ff");
        assert_eq!(decode::<4>("007f80ff"), Some(bytes));
    }

    #[test]
    fn accepts_uppercase() {
        assert_eq!(decode::<2>("ABcd"), Some([0xab, 0xcd]));
    }

    #[test]
    fn refuses_wrong_length_and_bad_digits() {
        assert_eq!(decode::<2>("abc"), None);
        assert_eq!(decode::<2>("abcdef"), None);
        assert_eq!(decode::<2>("abcg"), None);
        assert_eq!(decode::<1>("+1"), None);
    }
}
