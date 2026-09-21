//! The four sides of a box, and the `margin` and `padding` shorthands.

use crate::values::length::{Length, parse_length};

/// A value for each side of a box, in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Edges {
    /// Top.
    pub top: f32,
    /// Right.
    pub right: f32,
    /// Bottom.
    pub bottom: f32,
    /// Left.
    pub left: f32,
}

impl Edges {
    /// The same value on every side.
    pub const fn all(v: f32) -> Self {
        Self::new(v, v, v, v)
    }

    /// `v` above and below, nothing left and right.
    pub const fn vertical(v: f32) -> Self {
        Self::new(v, 0.0, v, 0.0)
    }

    /// `v` above and below, `h` left and right.
    pub const fn symmetric(v: f32, h: f32) -> Self {
        Self::new(v, h, v, h)
    }

    /// Top, right, bottom, left, as CSS lists them.
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }
}

/// One to four lengths in CSS order (top, right, bottom, left), expanded to
/// all four sides as the `margin` and `padding` shorthands do.
pub fn parse_edges(parts: &[&str], font_size: f32) -> Option<[Length; 4]> {
    let v: Vec<Length> = parts
        .iter()
        .map(|p| parse_length(p, font_size))
        .collect::<Option<_>>()?;
    match v.as_slice() {
        [a] => Some([*a, *a, *a, *a]),
        [a, b] => Some([*a, *b, *a, *b]),
        [a, b, c] => Some([*a, *b, *c, *b]),
        [a, b, c, d] => Some([*a, *b, *c, *d]),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Length::{Auto, Px};

    #[test]
    fn shorthand_expands_like_css() {
        assert_eq!(parse_edges(&["1px"], 16.0), Some([Px(1.0); 4]));
        assert_eq!(
            parse_edges(&["0", "auto"], 16.0),
            Some([Px(0.0), Auto, Px(0.0), Auto])
        );
        assert_eq!(
            parse_edges(&["1px", "2px", "3px"], 16.0),
            Some([Px(1.0), Px(2.0), Px(3.0), Px(2.0)])
        );
        assert_eq!(
            parse_edges(&["1px", "2px", "3px", "4px"], 16.0),
            Some([Px(1.0), Px(2.0), Px(3.0), Px(4.0)])
        );
        assert_eq!(parse_edges(&[], 16.0), None);
        assert_eq!(parse_edges(&["1px"; 5], 16.0), None);
        assert_eq!(parse_edges(&["1px", "big"], 16.0), None);
    }
}
