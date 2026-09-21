//! Breaking inline content into lines: fill each line left to right, start a
//! new line when the next piece does not fit, then align each line.

use vpp_style::TextAlign;

use crate::geometry::Rect;

/// One piece of inline content, measured.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Measured {
    /// Its width, including an inline-level box's horizontal margins.
    pub(crate) width: f32,
    /// The distance from its top to its baseline.
    pub(crate) ascent: f32,
    /// Its full height.
    pub(crate) height: f32,
    /// The width of the space before it, if one separates it from the previous piece.
    pub(crate) space_before: f32,
}

/// A line, relative to the top-left of its content area.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedLine {
    /// The line's box after alignment.
    pub(crate) rect: Rect,
    /// Each piece on the line: its index in the input, its box, and the distance from the box's top to the baseline.
    pub(crate) pieces: Vec<(usize, Rect, f32)>,
}

/// Places `items` in lines at most `width` wide, aligned by `align`. A piece
/// wider than the line gets a line of its own. Returns the lines and their total height.
pub(crate) fn break_lines(
    items: &[Measured],
    width: f32,
    align: TextAlign,
) -> (Vec<PlacedLine>, f32) {
    let mut lines = Vec::new();
    let mut y = 0.0;
    let mut i = 0;
    while i < items.len() {
        let mut members = Vec::new();
        let mut x = 0.0;
        let (mut ascent, mut descent) = (0.0f32, 0.0f32);
        while let Some(item) = items.get(i) {
            let space = if members.is_empty() {
                0.0
            } else {
                item.space_before
            };
            // A small tolerance keeps rounding from pushing an exact fit onto the next line.
            if !members.is_empty() && x + space + item.width > width + 0.01 {
                break;
            }
            members.push((i, x + space));
            x += space + item.width;
            ascent = ascent.max(item.ascent);
            descent = descent.max(item.height - item.ascent);
            i += 1;
        }

        let shift = if width.is_finite() && width > x {
            match align {
                TextAlign::Left => 0.0,
                TextAlign::Center => (width - x) / 2.0,
                TextAlign::Right => width - x,
            }
        } else {
            0.0
        };
        let height = ascent + descent;
        let pieces = members
            .into_iter()
            .map(|(index, left)| {
                let item = &items[index];
                let rect = Rect::new(
                    shift + left,
                    y + ascent - item.ascent,
                    item.width,
                    item.height,
                );
                (index, rect, item.ascent)
            })
            .collect();
        lines.push(PlacedLine {
            rect: Rect::new(shift, y, x, height),
            pieces,
        });
        y += height;
    }
    (lines, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(width: f32) -> Measured {
        Measured {
            width,
            ascent: 16.0,
            height: 20.0,
            space_before: 8.0,
        }
    }

    #[test]
    fn fills_lines_then_wraps() {
        let items = [word(40.0), word(40.0), word(40.0)];
        let (lines, height) = break_lines(&items, 100.0, TextAlign::Left);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].rect.w, 88.0); // 40 + space 8 + 40
        assert_eq!(lines[0].pieces[1].1.x, 48.0);
        assert_eq!(lines[1].pieces[0].1, Rect::new(0.0, 20.0, 40.0, 20.0));
        assert_eq!(height, 40.0);
    }

    #[test]
    fn a_piece_wider_than_the_line_still_gets_one() {
        let (lines, _) = break_lines(&[word(150.0), word(10.0)], 100.0, TextAlign::Left);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].rect.w, 150.0);
    }

    #[test]
    fn alignment_shifts_the_line() {
        let (center, _) = break_lines(&[word(40.0)], 100.0, TextAlign::Center);
        assert_eq!(center[0].pieces[0].1.x, 30.0);
        let (right, _) = break_lines(&[word(40.0)], 100.0, TextAlign::Right);
        assert_eq!(right[0].pieces[0].1.x, 60.0);
    }

    #[test]
    fn mixed_heights_share_one_baseline() {
        let tall = Measured {
            width: 10.0,
            ascent: 30.0,
            height: 40.0,
            space_before: 0.0,
        };
        let (lines, height) = break_lines(&[word(10.0), tall], 100.0, TextAlign::Left);
        let (_, small, small_baseline) = lines[0].pieces[0];
        let (_, big, big_baseline) = lines[0].pieces[1];
        assert_eq!(small.y + small_baseline, big.y + big_baseline);
        assert_eq!(height, 30.0 + 10.0_f32.max(4.0));
    }

    #[test]
    fn unlimited_width_is_one_line() {
        let items = [word(1000.0), word(1000.0)];
        let (lines, _) = break_lines(&items, f32::INFINITY, TextAlign::Center);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].rect.x, 0.0);
    }
}
