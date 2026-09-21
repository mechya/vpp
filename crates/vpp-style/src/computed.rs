//! The computed style of one element: every value layout and painting need,
//! resolved to numbers and keywords. All lengths are CSS pixels.

use crate::values::color::Color;
use crate::values::edges::Edges;
use crate::values::length::Length;

/// How an element takes part in layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    /// Not rendered, and its descendants neither.
    None,
    /// A block box.
    Block,
    /// Inline content.
    #[default]
    Inline,
    /// A block box placed in a line.
    InlineBlock,
    /// A flex container.
    Flex,
}

/// Horizontal alignment of lines inside a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Start of the line.
    #[default]
    Left,
    /// Centred.
    Center,
    /// End of the line.
    Right,
}

/// The main axis of a flex container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// Left to right.
    #[default]
    Row,
    /// Top to bottom.
    Column,
}

/// Distribution of flex items along the main axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JustifyContent {
    /// Packed at the start.
    #[default]
    Start,
    /// Packed in the middle.
    Center,
    /// Packed at the end.
    End,
    /// Even gaps between items.
    SpaceBetween,
    /// Even space around each item.
    SpaceAround,
}

/// Alignment of flex items on the cross axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignItems {
    /// Stretched to fill the line.
    #[default]
    Stretch,
    /// At the start.
    Start,
    /// Centred.
    Center,
    /// At the end.
    End,
}

/// The resolved style of one element (`docs/reference/runtime.md`, "Supported CSS").
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    /// How the element takes part in layout.
    pub display: Display,

    /// Text colour. Inherited.
    pub color: Color,
    /// Font size in CSS pixels. Inherited.
    pub font_size: f32,
    /// Bold text (`font-weight` 600 or more). Inherited.
    pub bold: bool,
    /// Line alignment. Inherited.
    pub text_align: TextAlign,

    /// Background colour; transparent draws nothing.
    pub background: Color,
    /// Margins, with `auto` sides as 0 and flagged below.
    pub margin: Edges,
    /// Whether the left margin is `auto`.
    pub margin_left_auto: bool,
    /// Whether the right margin is `auto`.
    pub margin_right_auto: bool,
    /// Padding.
    pub padding: Edges,
    /// Border width, the same on every side.
    pub border_width: f32,
    /// Border colour.
    pub border_color: Color,
    /// Corner radius, the same on every corner.
    pub border_radius: f32,
    /// Content-box width.
    pub width: Length,
    /// Content-box height.
    pub height: Length,
    /// Largest content-box width; `auto` means none.
    pub max_width: Length,

    /// Main axis, for a flex container.
    pub flex_direction: FlexDirection,
    /// Main-axis distribution, for a flex container.
    pub justify_content: JustifyContent,
    /// Cross-axis alignment, for a flex container.
    pub align_items: AlignItems,
    /// Space between flex items.
    pub gap: f32,

    /// How much of the free space this flex item takes.
    pub flex_grow: f32,
}

impl ComputedStyle {
    /// The style above the root element, before any element is considered.
    pub fn initial() -> Self {
        Self {
            display: Display::Inline,
            color: Color::rgb(24, 24, 27),
            font_size: 16.0,
            bold: false,
            text_align: TextAlign::Left,
            background: Color::TRANSPARENT,
            margin: Edges::default(),
            margin_left_auto: false,
            margin_right_auto: false,
            padding: Edges::default(),
            border_width: 0.0,
            border_color: Color::rgb(0, 0, 0),
            border_radius: 0.0,
            width: Length::Auto,
            height: Length::Auto,
            max_width: Length::Auto,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            gap: 0.0,
            flex_grow: 0.0,
        }
    }

    /// A fresh style for a child: inherited properties from `self`, everything else initial.
    pub(crate) fn inherit(&self) -> Self {
        Self {
            color: self.color,
            font_size: self.font_size,
            bold: self.bold,
            text_align: self.text_align,
            ..Self::initial()
        }
    }

    /// Whether a background is drawn.
    pub fn has_background(&self) -> bool {
        self.background.a > 0
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}
