//! A computed style in `taffy`'s terms. Widths and heights are content-box
//! sizes, as in CSS by default and the C++ engine.

use taffy::geometry::{Rect, Size};
use taffy::prelude::{Dimension, LengthPercentage, LengthPercentageAuto};
use taffy::style::{AlignItems, BoxSizing, Display, FlexDirection, JustifyContent, Style};
use vpp_style::{ComputedStyle, Length};

/// The `taffy` style for a box with this computed style. `display` is the
/// layout `taffy` should run for it: block or flex.
pub(crate) fn taffy_style(s: &ComputedStyle, display: Display) -> Style {
    let side = |px: f32| LengthPercentage::length(px);
    let border = side(s.border_width);
    Style {
        display,
        box_sizing: BoxSizing::ContentBox,
        size: Size {
            width: dimension(s.width),
            height: dimension(s.height),
        },
        max_size: Size {
            width: match s.max_width {
                Length::Auto => LengthPercentageAuto::auto(),
                Length::Px(px) => LengthPercentageAuto::length(px),
                Length::Percent(p) => LengthPercentageAuto::percent(p / 100.0),
            },
            height: LengthPercentageAuto::auto(),
        },
        margin: Rect {
            left: margin(s.margin.left, s.margin_left_auto),
            right: margin(s.margin.right, s.margin_right_auto),
            top: LengthPercentageAuto::length(s.margin.top),
            bottom: LengthPercentageAuto::length(s.margin.bottom),
        },
        padding: Rect {
            left: side(s.padding.left),
            right: side(s.padding.right),
            top: side(s.padding.top),
            bottom: side(s.padding.bottom),
        },
        border: Rect {
            left: border,
            right: border,
            top: border,
            bottom: border,
        },
        flex_direction: match s.flex_direction {
            vpp_style::FlexDirection::Row => FlexDirection::Row,
            vpp_style::FlexDirection::Column => FlexDirection::Column,
        },
        justify_content: Some(match s.justify_content {
            vpp_style::JustifyContent::Start => JustifyContent::FLEX_START,
            vpp_style::JustifyContent::Center => JustifyContent::CENTER,
            vpp_style::JustifyContent::End => JustifyContent::FLEX_END,
            vpp_style::JustifyContent::SpaceBetween => JustifyContent::SPACE_BETWEEN,
            vpp_style::JustifyContent::SpaceAround => JustifyContent::SPACE_AROUND,
        }),
        align_items: Some(match s.align_items {
            vpp_style::AlignItems::Stretch => AlignItems::STRETCH,
            vpp_style::AlignItems::Start => AlignItems::FLEX_START,
            vpp_style::AlignItems::Center => AlignItems::CENTER,
            vpp_style::AlignItems::End => AlignItems::FLEX_END,
        }),
        gap: Size {
            width: side(s.gap),
            height: side(s.gap),
        },
        flex_grow: s.flex_grow,
        ..Style::default()
    }
}

/// The style of a box that only holds a run of inline content.
pub(crate) fn anonymous_block() -> Style {
    Style {
        display: Display::Block,
        ..Style::default()
    }
}

fn dimension(length: Length) -> Dimension {
    match length {
        Length::Auto => Dimension::auto(),
        Length::Px(px) => Dimension::length(px),
        Length::Percent(p) => Dimension::percent(p / 100.0),
    }
}

fn margin(px: f32, auto: bool) -> LengthPercentageAuto {
    if auto {
        LengthPercentageAuto::auto()
    } else {
        LengthPercentageAuto::length(px)
    }
}
