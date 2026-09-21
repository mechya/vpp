//! Measuring text: the one thing layout needs from fonts.
//!
//! Layout asks, and a font answers. `vpp-paint` answers with real fonts
//! through `swash`; tests answer with fixed sizes, so layout tests do not
//! depend on any font.

/// Sizes of text in CSS pixels, for a font size and weight.
pub trait TextMeasure {
    /// The advance width of `text` on one line.
    fn width(&self, text: &str, font_size: f32, bold: bool) -> f32;

    /// The distance from the top of a line to its baseline.
    fn ascent(&self, font_size: f32, bold: bool) -> f32;

    /// The full height of a line of text.
    fn line_height(&self, font_size: f32, bold: bool) -> f32;
}

/// A font where every character is `0.5 × font_size` wide and lines are
/// `1.25 × font_size` tall, with the baseline at `font_size`. For tests and
/// tools that need layout without a real font.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedMeasure;

impl TextMeasure for FixedMeasure {
    fn width(&self, text: &str, font_size: f32, _bold: bool) -> f32 {
        text.chars().count() as f32 * font_size * 0.5
    }

    fn ascent(&self, font_size: f32, _bold: bool) -> f32 {
        font_size
    }

    fn line_height(&self, font_size: f32, _bold: bool) -> f32 {
        font_size * 1.25
    }
}
