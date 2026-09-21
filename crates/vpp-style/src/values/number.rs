//! Numbers at the start of a CSS value, and splitting a value into its parts.

/// The number at the start of `text` and the byte length it took, for
/// example `12.5` of `12.5px`. `None` if `text` does not start with one.
pub(crate) fn parse_number(text: &str) -> Option<(f32, usize)> {
    let bytes = text.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'-' | b'+')));
    let mut seen_digit = false;
    let mut seen_dot = false;
    while let Some(&b) = bytes.get(end) {
        match b {
            b'0'..=b'9' => seen_digit = true,
            b'.' if !seen_dot => seen_dot = true,
            _ => break,
        }
        end += 1;
    }
    if !seen_digit {
        return None;
    }
    Some((text[..end].parse().ok()?, end))
}

/// A value's space-separated parts, keeping each function call such as
/// `rgb(1, 2, 3)` whole.
pub(crate) fn tokens(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    for (i, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c.is_ascii_whitespace() && depth <= 0 {
            if let Some(s) = start.take() {
                out.push(&value[s..i]);
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push(&value[s..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_where_they_end() {
        assert_eq!(parse_number("12px"), Some((12.0, 2)));
        assert_eq!(parse_number("-1.5em"), Some((-1.5, 4)));
        assert_eq!(parse_number(".5"), Some((0.5, 2)));
        assert_eq!(parse_number("1.2.3"), Some((1.2, 3)));
        assert_eq!(parse_number("px"), None);
        assert_eq!(parse_number("-"), None);
    }

    #[test]
    fn tokens_keep_functions_whole() {
        assert_eq!(
            tokens("1px solid rgb(1, 2, 3)"),
            ["1px", "solid", "rgb(1, 2, 3)"]
        );
        assert_eq!(tokens("  0   auto "), ["0", "auto"]);
        assert!(tokens("   ").is_empty());
    }
}
