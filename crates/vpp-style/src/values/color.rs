//! CSS colours: named colours, `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`,
//! `rgb()`, `rgba()`, and `transparent` (`docs/reference/runtime.md`).

use crate::values::number::parse_number;

/// An sRGB colour with alpha, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha: 0 is transparent, 255 opaque.
    pub a: u8,
}

impl Color {
    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// A colour with alpha.
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
}

const NAMED: &[(&str, Color)] = &[
    ("black", Color::rgb(0, 0, 0)),
    ("white", Color::rgb(255, 255, 255)),
    ("red", Color::rgb(255, 0, 0)),
    ("green", Color::rgb(0, 128, 0)),
    ("blue", Color::rgb(0, 0, 255)),
    ("yellow", Color::rgb(255, 255, 0)),
    ("orange", Color::rgb(255, 165, 0)),
    ("purple", Color::rgb(128, 0, 128)),
    ("pink", Color::rgb(255, 192, 203)),
    ("gray", Color::rgb(128, 128, 128)),
    ("grey", Color::rgb(128, 128, 128)),
    ("silver", Color::rgb(192, 192, 192)),
    ("navy", Color::rgb(0, 0, 128)),
    ("teal", Color::rgb(0, 128, 128)),
    ("maroon", Color::rgb(128, 0, 0)),
    ("olive", Color::rgb(128, 128, 0)),
    ("lime", Color::rgb(0, 255, 0)),
    ("aqua", Color::rgb(0, 255, 255)),
    ("cyan", Color::rgb(0, 255, 255)),
    ("fuchsia", Color::rgb(255, 0, 255)),
    ("magenta", Color::rgb(255, 0, 255)),
    ("brown", Color::rgb(165, 42, 42)),
    ("transparent", Color::TRANSPARENT),
];

/// Parses a CSS colour, in any letter case.
pub fn parse_color(text: &str) -> Option<Color> {
    let text = text.trim().to_ascii_lowercase();
    if let Some(&(_, color)) = NAMED.iter().find(|(name, _)| *name == text) {
        return Some(color);
    }
    if let Some(hex) = text.strip_prefix('#') {
        return parse_hex(hex);
    }
    let inner = text
        .strip_prefix("rgba(")
        .or_else(|| text.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    parse_rgb_arguments(inner)
}

/// `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`, without the `#`.
fn parse_hex(hex: &str) -> Option<Color> {
    let digits: Vec<u8> = hex
        .chars()
        .map(|c| c.to_digit(16).map(|d| d as u8))
        .collect::<Option<_>>()?;
    let pair = |i: usize| digits[i] * 16 + digits[i + 1];
    match digits.len() {
        3 | 4 => Some(Color {
            r: digits[0] * 17,
            g: digits[1] * 17,
            b: digits[2] * 17,
            a: digits.get(3).map_or(255, |a| a * 17),
        }),
        6 | 8 => Some(Color {
            r: pair(0),
            g: pair(2),
            b: pair(4),
            a: if digits.len() == 8 { pair(6) } else { 255 },
        }),
        _ => None,
    }
}

/// The arguments of `rgb()` or `rgba()`, separated by commas, spaces, or `/`.
/// Channels are 0–255 or percentages; alpha is 0–1, a percentage, or 0–255.
fn parse_rgb_arguments(inner: &str) -> Option<Color> {
    let parts: Vec<f32> = inner
        .split([',', '/', ' ', '\t', '\n'])
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let value = parse_number(part)?.0;
            Some(if part.ends_with('%') {
                value * 2.55
            } else {
                value
            })
        })
        .collect();
    if parts.len() < 3 {
        return None;
    }
    let channel = |v: f32| (v.clamp(0.0, 255.0) + 0.5) as u8;
    let alpha = parts
        .get(3)
        .map_or(255, |&a| channel(if a <= 1.0 { a * 255.0 } else { a }));
    Some(Color::rgba(
        channel(parts[0]),
        channel(parts[1]),
        channel(parts[2]),
        alpha,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_colours_in_any_case() {
        assert_eq!(parse_color("Red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(parse_color("transparent"), Some(Color::TRANSPARENT));
        assert_eq!(parse_color("notacolor"), None);
    }

    #[test]
    fn hex_forms() {
        assert_eq!(parse_color("#fff"), Some(Color::rgb(255, 255, 255)));
        assert_eq!(parse_color("#2563EB"), Some(Color::rgb(37, 99, 235)));
        assert_eq!(parse_color("#0008"), Some(Color::rgba(0, 0, 0, 136)));
        assert_eq!(parse_color("#11223344"), Some(Color::rgba(17, 34, 51, 68)));
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#ggg"), None);
    }

    #[test]
    fn rgb_and_rgba() {
        assert_eq!(parse_color("rgb(1, 2, 3)"), Some(Color::rgb(1, 2, 3)));
        assert_eq!(
            parse_color("rgba(0, 0, 0, 0.5)"),
            Some(Color::rgba(0, 0, 0, 128))
        );
        assert_eq!(
            parse_color("rgb(100% 0% 0% / 50%)"),
            Some(Color::rgba(255, 0, 0, 128))
        );
        assert_eq!(parse_color("rgb(300, -5, 7)"), Some(Color::rgb(255, 0, 7)));
        assert_eq!(parse_color("rgb(1, 2)"), None);
    }
}
