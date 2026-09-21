//! Lengths: `px`, `em`, `rem`, `%`, unitless numbers (as px), and `auto`.

use crate::values::number::parse_number;

/// The font size of the root element, which `rem` is relative to.
pub const ROOT_FONT_SIZE: f32 = 16.0;

/// A length as declared: absolute CSS pixels, a percentage of something
/// layout decides, or `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    /// Decided by layout.
    #[default]
    Auto,
    /// CSS pixels.
    Px(f32),
    /// A percentage of a reference length, such as the containing block's width.
    Percent(f32),
}

impl Length {
    /// Whether this is `auto`.
    pub fn is_auto(self) -> bool {
        self == Length::Auto
    }

    /// Pixels, resolving percentages against `reference` and `auto` to `fallback`.
    pub fn resolve(self, reference: f32, fallback: f32) -> f32 {
        match self {
            Length::Px(px) => px,
            Length::Percent(p) => reference * p / 100.0,
            Length::Auto => fallback,
        }
    }
}

/// Parses a length. `em` is relative to `font_size`, `rem` to [`ROOT_FONT_SIZE`].
pub fn parse_length(text: &str, font_size: f32) -> Option<Length> {
    let text = text.trim().to_ascii_lowercase();
    if text == "auto" {
        return Some(Length::Auto);
    }
    let (number, end) = parse_number(&text)?;
    match &text[end..] {
        "px" | "" => Some(Length::Px(number)),
        "em" => Some(Length::Px(number * font_size)),
        "rem" => Some(Length::Px(number * ROOT_FONT_SIZE)),
        "%" => Some(Length::Percent(number)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units() {
        assert_eq!(parse_length("12px", 16.0), Some(Length::Px(12.0)));
        assert_eq!(parse_length("0", 16.0), Some(Length::Px(0.0)));
        assert_eq!(parse_length("1.5em", 20.0), Some(Length::Px(30.0)));
        assert_eq!(parse_length("2rem", 99.0), Some(Length::Px(32.0)));
        assert_eq!(parse_length("50%", 16.0), Some(Length::Percent(50.0)));
        assert_eq!(parse_length("AUTO", 16.0), Some(Length::Auto));
        assert_eq!(parse_length("3vw", 16.0), None);
        assert_eq!(parse_length("wide", 16.0), None);
    }

    #[test]
    fn resolving() {
        assert_eq!(Length::Percent(50.0).resolve(300.0, 0.0), 150.0);
        assert_eq!(Length::Auto.resolve(300.0, 7.0), 7.0);
        assert_eq!(Length::Px(4.0).resolve(300.0, 0.0), 4.0);
    }
}
