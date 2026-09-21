//! Inline content: a run of words and inline-level boxes inside a block,
//! collected from the DOM and laid out into lines at a given width.
//!
//! Whitespace collapses: each run of spaces becomes one space between
//! pieces, and spaces at the start of a run are dropped.

use vpp_dom::{Document, NodeData, NodeId};
use vpp_style::{ComputedStyle, Display, StyleSheet, TextAlign, compute_style};

use crate::geometry::Rect;
use crate::layouter::Layouter;
use crate::line_break::{Measured, break_lines};
use crate::tree::{Fragment, FragmentContent, LayoutBox, Line};

/// A run of inline content, laid out by `vpp-layout` itself rather than `taffy`.
#[derive(Debug, Clone)]
pub(crate) struct InlineRun {
    pub(crate) items: Vec<InlineItem>,
    pub(crate) text_align: TextAlign,
}

#[derive(Debug, Clone)]
pub(crate) struct InlineItem {
    pub(crate) content: InlineContent,
    /// The style the piece is drawn with: the text's, or the box's own.
    pub(crate) style: ComputedStyle,
    pub(crate) space_before: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum InlineContent {
    Word(String),
    /// An inline-block, or a block-level element inside inline content.
    Box(NodeId),
}

/// A run laid out at one width, relative to the run's top-left corner.
pub(crate) struct LaidOutRun {
    pub(crate) lines: Vec<Line>,
    /// The inline-level boxes, which the lines' box fragments refer to by index.
    pub(crate) boxes: Vec<LayoutBox>,
    /// The widest line.
    pub(crate) content_width: f32,
    pub(crate) height: f32,
}

/// Collects inline content from the DOM into a run.
pub(crate) struct Collector<'a> {
    pub(crate) document: &'a Document,
    pub(crate) sheets: &'a [StyleSheet],
    pub(crate) items: Vec<InlineItem>,
    pending_space: bool,
}

impl<'a> Collector<'a> {
    pub(crate) fn new(document: &'a Document, sheets: &'a [StyleSheet]) -> Self {
        Self {
            document,
            sheets,
            items: Vec::new(),
            pending_space: false,
        }
    }

    /// Adds `node` and what is inside it. `parent_style` is the style of the
    /// element containing `node`.
    pub(crate) fn add(&mut self, node: NodeId, parent_style: &ComputedStyle) {
        match self.document[node].data() {
            NodeData::Text(text) => self.add_text(text, parent_style),
            NodeData::Element(_) => {
                let style = compute_style(self.document, node, parent_style, self.sheets);
                self.add_element(node, style);
            }
            NodeData::Document => {}
        }
    }

    /// Adds an element whose style is already computed.
    pub(crate) fn add_element(&mut self, node: NodeId, style: ComputedStyle) {
        match style.display {
            Display::None => {}
            Display::Inline => {
                for &child in self.document[node].children() {
                    self.add(child, &style);
                }
            }
            Display::InlineBlock | Display::Block | Display::Flex => {
                self.items.push(InlineItem {
                    content: InlineContent::Box(node),
                    style,
                    space_before: self.pending_space,
                });
                self.pending_space = false;
            }
        }
    }

    fn add_text(&mut self, text: &str, style: &ComputedStyle) {
        let mut rest = text;
        loop {
            let trimmed = rest.trim_start_matches(|c: char| c.is_ascii_whitespace());
            let had_space = trimmed.len() != rest.len();
            if trimmed.is_empty() {
                self.pending_space |= had_space;
                return;
            }
            let end = trimmed
                .find(|c: char| c.is_ascii_whitespace())
                .unwrap_or(trimmed.len());
            self.items.push(InlineItem {
                content: InlineContent::Word(trimmed[..end].to_owned()),
                style: style.clone(),
                space_before: had_space || self.pending_space,
            });
            self.pending_space = false;
            rest = &trimmed[end..];
        }
    }

    pub(crate) fn finish(self, text_align: TextAlign) -> Option<InlineRun> {
        (!self.items.is_empty()).then_some(InlineRun {
            items: self.items,
            text_align,
        })
    }
}

impl InlineRun {
    /// Lays out the run in lines at most `width` wide (`f32::INFINITY` for one line).
    pub(crate) fn lay_out(&self, layouter: &Layouter<'_>, width: f32) -> LaidOutRun {
        let measure = layouter.measure;
        let mut boxes = Vec::new();
        let mut box_index = Vec::with_capacity(self.items.len());
        let measured: Vec<Measured> = self
            .items
            .iter()
            .map(|item| {
                let s = &item.style;
                let space_before = if item.space_before {
                    measure.width(" ", s.font_size, s.bold)
                } else {
                    0.0
                };
                match &item.content {
                    InlineContent::Word(text) => {
                        box_index.push(None);
                        Measured {
                            width: measure.width(text, s.font_size, s.bold),
                            ascent: measure.ascent(s.font_size, s.bold),
                            height: measure.line_height(s.font_size, s.bold),
                            space_before,
                        }
                    }
                    InlineContent::Box(node) => {
                        let laid = layouter.lay_out_inline_block(*node, s, width);
                        let m = Measured {
                            width: laid.frame.w + s.margin.left + s.margin.right,
                            ascent: laid.baseline + s.margin.top,
                            height: laid.frame.h + s.margin.top + s.margin.bottom,
                            space_before,
                        };
                        box_index.push(Some(boxes.len()));
                        boxes.push(laid);
                        m
                    }
                }
            })
            .collect();

        let (placed, height) = break_lines(&measured, width, self.text_align);
        let content_width = placed.iter().map(|line| line.rect.w).fold(0.0, f32::max);
        let lines = placed
            .into_iter()
            .map(|line| Line {
                rect: line.rect,
                fragments: line
                    .pieces
                    .into_iter()
                    .map(|(index, rect, baseline)| {
                        let item = &self.items[index];
                        let content = match (&item.content, box_index[index]) {
                            (InlineContent::Word(text), _) => FragmentContent::Text {
                                text: text.clone(),
                                color: item.style.color,
                                font_size: item.style.font_size,
                                bold: item.style.bold,
                            },
                            (InlineContent::Box(_), Some(b)) => {
                                let s = &item.style;
                                boxes[b].translate(rect.x + s.margin.left, rect.y + s.margin.top);
                                FragmentContent::Box(b)
                            }
                            (InlineContent::Box(_), None) => unreachable!("every box was laid out"),
                        };
                        Fragment {
                            rect,
                            baseline,
                            content,
                        }
                    })
                    .collect(),
            })
            .collect();
        LaidOutRun {
            lines,
            boxes,
            content_width,
            height,
        }
    }
}

/// The style of the anonymous box around a run: text properties from its
/// parent, nothing else.
pub(crate) fn anonymous_style(parent: &ComputedStyle) -> ComputedStyle {
    ComputedStyle {
        display: Display::Block,
        color: parent.color,
        font_size: parent.font_size,
        bold: parent.bold,
        text_align: parent.text_align,
        ..ComputedStyle::initial()
    }
}

/// Moves every piece of a laid-out run to `origin`.
pub(crate) fn place(run: &mut LaidOutRun, origin: Rect) {
    for line in &mut run.lines {
        line.rect = line.rect.translated(origin.x, origin.y);
        for fragment in &mut line.fragments {
            fragment.rect = fragment.rect.translated(origin.x, origin.y);
        }
    }
    for b in &mut run.boxes {
        b.translate(origin.x, origin.y);
    }
}
