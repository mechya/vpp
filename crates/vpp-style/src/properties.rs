//! The property table: how each supported declaration changes a computed
//! style. Adding a CSS property means a value parser in `values/` (if it needs
//! a new kind of value) and one arm here. Unknown properties and values that
//! do not parse are ignored, as in a browser.

use crate::computed::{
    AlignItems, ComputedStyle, Display, FlexDirection, JustifyContent, TextAlign,
};
use crate::stylesheet::Declaration;
use crate::values::color::parse_color;
use crate::values::edges::{Edges, parse_edges};
use crate::values::length::{Length, parse_length};
use crate::values::number::{parse_number, tokens};

/// Applies one declaration to `s`. `parent` is the parent's computed style,
/// for `em` in `font-size`.
pub(crate) fn apply(declaration: &Declaration, parent: &ComputedStyle, s: &mut ComputedStyle) {
    let value = declaration.value.to_ascii_lowercase();
    let parts = tokens(&value);
    let Some(&first) = parts.first() else {
        return;
    };
    let length = |text: &str, s: &ComputedStyle| parse_length(text, s.font_size);

    match declaration.property.as_str() {
        "display" => {
            s.display = match value.as_str() {
                "none" => Display::None,
                "block" => Display::Block,
                "inline" => Display::Inline,
                "inline-block" => Display::InlineBlock,
                "flex" => Display::Flex,
                _ => return,
            }
        }
        "color" => {
            if let Some(color) = parse_color(&value) {
                s.color = color;
            }
        }
        "background-color" | "background" => {
            if let Some(color) = parts.iter().find_map(|p| parse_color(p)) {
                s.background = color;
            }
        }
        "font-size" => {
            if let Some(size) = parse_length(&value, parent.font_size) {
                s.font_size = size.resolve(parent.font_size, parent.font_size);
            }
        }
        "font-weight" => {
            s.bold = match value.as_str() {
                "bold" | "bolder" => true,
                "normal" | "lighter" => false,
                _ => match parse_number(&value) {
                    Some((weight, _)) => weight >= 600.0,
                    None => return,
                },
            }
        }
        "text-align" => {
            s.text_align = match value.as_str() {
                "left" | "start" => TextAlign::Left,
                "center" => TextAlign::Center,
                "right" | "end" => TextAlign::Right,
                _ => return,
            }
        }
        "margin" => {
            if let Some([top, right, bottom, left]) = parse_edges(&parts, s.font_size) {
                s.margin = Edges::new(px(top), px(right), px(bottom), px(left));
                s.margin_left_auto = left.is_auto();
                s.margin_right_auto = right.is_auto();
            }
        }
        "margin-top" | "margin-right" | "margin-bottom" | "margin-left" => {
            if let Some(len) = length(&value, s) {
                match declaration.property.as_str() {
                    "margin-top" => s.margin.top = px(len),
                    "margin-bottom" => s.margin.bottom = px(len),
                    "margin-left" => {
                        s.margin.left = px(len);
                        s.margin_left_auto = len.is_auto();
                    }
                    _ => {
                        s.margin.right = px(len);
                        s.margin_right_auto = len.is_auto();
                    }
                }
            }
        }
        "padding" => {
            if let Some([top, right, bottom, left]) = parse_edges(&parts, s.font_size) {
                s.padding = Edges::new(px(top), px(right), px(bottom), px(left));
            }
        }
        "padding-top" | "padding-right" | "padding-bottom" | "padding-left" => {
            if let Some(len) = length(&value, s) {
                let side = match declaration.property.as_str() {
                    "padding-top" => &mut s.padding.top,
                    "padding-right" => &mut s.padding.right,
                    "padding-bottom" => &mut s.padding.bottom,
                    _ => &mut s.padding.left,
                };
                *side = px(len);
            }
        }
        "border" => {
            if value == "none" || value == "0" {
                s.border_width = 0.0;
                return;
            }
            for part in &parts {
                if let Some(len) = length(part, s) {
                    s.border_width = px(len);
                } else if let Some(color) = parse_color(part) {
                    s.border_color = color;
                }
                // "solid" and other border styles are accepted and ignored.
            }
            // "border: solid red" draws a 1px border, as in a browser.
            if s.border_width == 0.0 && s.border_color.a > 0 {
                s.border_width = 1.0;
            }
        }
        "border-width" => {
            if let Some(len) = length(&value, s) {
                s.border_width = px(len);
            }
        }
        "border-color" => {
            if let Some(color) = parse_color(&value) {
                s.border_color = color;
            }
        }
        "border-radius" => {
            if let Some(len) = length(first, s) {
                s.border_radius = px(len);
            }
        }
        "width" => {
            if let Some(len) = length(&value, s) {
                s.width = len;
            }
        }
        "height" => {
            if let Some(len) = length(&value, s) {
                s.height = len;
            }
        }
        "max-width" => {
            if value == "none" {
                s.max_width = Length::Auto;
            } else if let Some(len) = length(&value, s) {
                s.max_width = len;
            }
        }
        "flex-direction" => {
            s.flex_direction = match value.as_str() {
                "row" => FlexDirection::Row,
                "column" => FlexDirection::Column,
                _ => return,
            }
        }
        "justify-content" => {
            s.justify_content = match value.as_str() {
                "flex-start" | "start" | "left" => JustifyContent::Start,
                "center" => JustifyContent::Center,
                "flex-end" | "end" | "right" => JustifyContent::End,
                "space-between" => JustifyContent::SpaceBetween,
                "space-around" | "space-evenly" => JustifyContent::SpaceAround,
                _ => return,
            }
        }
        "align-items" => {
            s.align_items = match value.as_str() {
                "stretch" => AlignItems::Stretch,
                "flex-start" | "start" => AlignItems::Start,
                "center" => AlignItems::Center,
                "flex-end" | "end" => AlignItems::End,
                _ => return,
            }
        }
        "gap" => {
            if let Some(len) = length(first, s) {
                s.gap = px(len);
            }
        }
        "flex-grow" => {
            if let Some((grow, _)) = parse_number(&value) {
                s.flex_grow = grow.max(0.0);
            }
        }
        "flex" => {
            s.flex_grow = match value.as_str() {
                "none" | "initial" => 0.0,
                "auto" => 1.0,
                _ => match parse_number(first) {
                    Some((grow, _)) => grow.max(0.0),
                    None => return,
                },
            }
        }
        _ => {}
    }
}

/// Pixels for a box side: percentages and `auto` count as 0 here, as in the
/// C++ engine; `auto` margins are flagged separately.
fn px(length: Length) -> f32 {
    length.resolve(0.0, 0.0)
}
