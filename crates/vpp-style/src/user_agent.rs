//! The built-in default styles for HTML elements, applied before any page
//! stylesheet. Kept to VPP's own small set, the same as the C++ engine.

use crate::computed::{ComputedStyle, Display, TextAlign};
use crate::values::color::Color;
use crate::values::edges::Edges;

/// The accent colour of buttons.
const ACCENT: Color = Color::rgb(37, 99, 235);

/// The default style of an element with lower-case tag name `tag`.
pub(crate) fn user_agent_style(tag: &str, parent: &ComputedStyle) -> ComputedStyle {
    let mut s = parent.inherit();
    match tag {
        "html" => s.display = Display::Block,
        "body" => {
            s.display = Display::Block;
            s.margin = Edges::all(16.0);
        }
        "head" | "title" | "script" | "style" | "meta" | "link" => s.display = Display::None,
        "h1" => heading(&mut s, 32.0, 21.0),
        "h2" => heading(&mut s, 24.0, 20.0),
        "h3" => heading(&mut s, 19.0, 19.0),
        "p" => {
            s.display = Display::Block;
            s.margin = Edges::vertical(16.0);
        }
        "div" | "section" | "main" | "header" | "footer" | "nav" | "article" | "ul" | "ol"
        | "li" | "form" | "aside" => s.display = Display::Block,
        "button" => {
            s.display = Display::InlineBlock;
            s.color = Color::rgb(255, 255, 255);
            s.bold = false;
            s.background = ACCENT;
            s.padding = Edges::symmetric(12.0, 24.0);
            s.border_radius = 8.0;
            s.text_align = TextAlign::Center;
        }
        "b" | "strong" => s.bold = true,
        // A replaced element: sized by its attributes or CSS, painted as paths.
        "svg" => s.display = Display::InlineBlock,
        "path" | "defs" | "desc" => s.display = Display::None,
        _ => s.display = Display::Inline,
    }
    s
}

fn heading(s: &mut ComputedStyle, font_size: f32, margin: f32) {
    s.display = Display::Block;
    s.font_size = font_size;
    s.bold = true;
    s.margin = Edges::vertical(margin);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_inlines_and_hidden_elements() {
        let root = ComputedStyle::initial();
        assert_eq!(user_agent_style("div", &root).display, Display::Block);
        assert_eq!(user_agent_style("span", &root).display, Display::Inline);
        assert_eq!(user_agent_style("script", &root).display, Display::None);
        assert_eq!(
            user_agent_style("button", &root).display,
            Display::InlineBlock
        );
    }

    #[test]
    fn inherits_text_properties_only() {
        let mut parent = ComputedStyle::initial();
        parent.color = Color::rgb(1, 2, 3);
        parent.font_size = 20.0;
        parent.padding = Edges::all(9.0);
        let child = user_agent_style("span", &parent);
        assert_eq!(child.color, Color::rgb(1, 2, 3));
        assert_eq!(child.font_size, 20.0);
        assert_eq!(child.padding, Edges::default());
    }

    #[test]
    fn headings_are_bold_blocks() {
        let h1 = user_agent_style("h1", &ComputedStyle::initial());
        assert_eq!(
            (h1.display, h1.font_size, h1.bold),
            (Display::Block, 32.0, true)
        );
    }
}
