//! Fonts: loading TrueType and OpenType fonts with `swash`, and measuring text
//! for layout ([`TextMeasure`]). Drawing glyphs comes with painting.
//!
//! Text is measured by adding up each character's advance width, with no
//! kerning or shaping yet; drawing uses the same advances, so text always
//! fits the space layout gave it. Shaping for complex scripts comes with
//! `rustybuzz` (`docs/rust-port.md` §3).

use std::path::Path;
use std::sync::Arc;

use swash::FontRef;
use thiserror::Error;
use vpp_layout::TextMeasure;

/// Why a font could not be loaded.
#[derive(Debug, Error)]
pub enum FontError {
    /// The file could not be read.
    #[error("cannot read font {path}: {source}")]
    Io {
        /// The font file.
        path: String,
        /// What went wrong.
        source: std::io::Error,
    },
    /// The data is not a font `swash` can read.
    #[error("{0} is not a TrueType or OpenType font")]
    NotAFont(String),
}

/// One loaded font face.
#[derive(Clone)]
pub struct Font {
    data: Arc<[u8]>,
}

impl Font {
    /// A font from the bytes of a `.ttf` or `.otf` file. `name` is used in errors.
    pub fn from_bytes(data: Vec<u8>, name: &str) -> Result<Self, FontError> {
        FontRef::from_index(&data, 0).ok_or_else(|| FontError::NotAFont(name.to_owned()))?;
        Ok(Self { data: data.into() })
    }

    /// A font read from a file.
    pub fn load(path: &Path) -> Result<Self, FontError> {
        let name = path.display().to_string();
        let data = std::fs::read(path).map_err(|source| FontError::Io {
            path: name.clone(),
            source,
        })?;
        Self::from_bytes(data, &name)
    }

    /// The `swash` view of the font. Checked readable when the font was loaded.
    pub(crate) fn face(&self) -> FontRef<'_> {
        FontRef::from_index(&self.data, 0).expect("checked when the font was loaded")
    }

    /// The width of `text` at `size` pixels.
    pub fn width(&self, text: &str, size: f32) -> f32 {
        let face = self.face();
        let charmap = face.charmap();
        let glyphs = face.glyph_metrics(&[]).scale(size);
        text.chars()
            .map(|c| glyphs.advance_width(charmap.map(c)))
            .sum()
    }

    /// The distance from the top of a line to the baseline, at `size` pixels.
    pub fn ascent(&self, size: f32) -> f32 {
        self.face().metrics(&[]).scale(size).ascent
    }

    /// The height of a line, at `size` pixels: ascent, descent, and line gap.
    pub fn line_height(&self, size: f32) -> f32 {
        let m = self.face().metrics(&[]).scale(size);
        m.ascent + m.descent + m.leading
    }
}

impl std::fmt::Debug for Font {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Font({} bytes)", self.data.len())
    }
}

/// A regular face and an optional bold one. Bold text uses the regular face
/// when there is no bold one, as in the C++ engine.
#[derive(Debug, Clone)]
pub struct FontSet {
    /// The regular face.
    pub regular: Font,
    /// The bold face, if there is one.
    pub bold: Option<Font>,
}

impl FontSet {
    /// The face for bold or regular text.
    pub fn pick(&self, bold: bool) -> &Font {
        match (&self.bold, bold) {
            (Some(face), true) => face,
            _ => &self.regular,
        }
    }
}

impl TextMeasure for FontSet {
    fn width(&self, text: &str, font_size: f32, bold: bool) -> f32 {
        self.pick(bold).width(text, font_size)
    }

    fn ascent(&self, font_size: f32, bold: bool) -> f32 {
        self.pick(bold).ascent(font_size)
    }

    fn line_height(&self, font_size: f32, bold: bool) -> f32 {
        self.pick(bold).line_height(font_size)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// DejaVu Sans, the fonts tests use (`tests/fixtures/fonts/`).
    pub(crate) fn test_fonts() -> FontSet {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/fonts");
        FontSet {
            regular: Font::load(&dir.join("DejaVuSans.ttf")).unwrap(),
            bold: Some(Font::load(&dir.join("DejaVuSans-Bold.ttf")).unwrap()),
        }
    }

    #[test]
    fn widths_add_up_and_scale_with_size() {
        let fonts = test_fonts();
        let a = fonts.width("a", 16.0, false);
        assert!(a > 5.0 && a < 12.0, "{a}");
        assert!((fonts.width("aa", 16.0, false) - 2.0 * a).abs() < 0.001);
        assert!((fonts.width("a", 32.0, false) - 2.0 * a).abs() < 0.001);
        assert_eq!(fonts.width("", 16.0, false), 0.0);
    }

    #[test]
    fn bold_is_wider_and_falls_back_to_regular() {
        let fonts = test_fonts();
        assert!(fonts.width("Hello", 16.0, true) > fonts.width("Hello", 16.0, false));
        let regular_only = FontSet {
            regular: fonts.regular.clone(),
            bold: None,
        };
        assert_eq!(
            regular_only.width("Hello", 16.0, true),
            fonts.width("Hello", 16.0, false)
        );
    }

    #[test]
    fn line_metrics_are_plausible() {
        let fonts = test_fonts();
        let ascent = fonts.ascent(16.0, false);
        let line = fonts.line_height(16.0, false);
        assert!(ascent > 12.0 && ascent < 17.0, "{ascent}");
        assert!(line > ascent && line < 24.0, "{line}");
    }

    #[test]
    fn non_fonts_are_refused() {
        assert!(matches!(
            Font::from_bytes(b"not a font".to_vec(), "x"),
            Err(FontError::NotAFont(_))
        ));
    }

    #[test]
    fn example_pages_lay_out_with_real_fonts() {
        let doc = vpp_dom::parse_html("<p>Hello, VPP</p><button>Click me</button>");
        let root = vpp_layout::layout_document(&doc, &[], &test_fonts(), 800.0).unwrap();
        assert!(root.frame.h > 40.0);
    }
}
