//! Building a `taffy` tree from the DOM, running layout, and reading the
//! result back as a [`LayoutBox`] tree.
//!
//! Elements become `taffy` block or flex nodes. Each run of inline content
//! becomes a leaf that `taffy` asks to measure; `inline.rs` answers by
//! breaking the run into lines. Inline-level boxes inside a run (buttons,
//! icons) are laid out on their own, shrink-to-fit, and placed in the lines.

use taffy::geometry::Size;
use taffy::prelude::{AvailableSpace, NodeId as TaffyId, TaffyTree};
use taffy::style::Display as TaffyDisplay;
use taffy::{Style, compute_leaf_layout};
use vpp_dom::{Document, NodeData, NodeId};
use vpp_style::{ComputedStyle, Display, StyleSheet, compute_style, replaced_size};

use crate::geometry::Rect;
use crate::inline::{Collector, InlineRun, anonymous_style, place};
use crate::measure::TextMeasure;
use crate::taffy_style::{anonymous_block, taffy_style};
use crate::tree::LayoutBox;

/// What each `taffy` node stands for.
enum Context {
    /// An element, with its computed style.
    Element(NodeId, ComputedStyle),
    /// A run of inline content inside a block, with the block's style.
    Inline(InlineRun, ComputedStyle),
}

/// Everything layout reads: the document, its stylesheets, and a way to measure text.
pub(crate) struct Layouter<'a> {
    pub(crate) document: &'a Document,
    pub(crate) sheets: &'a [StyleSheet],
    pub(crate) measure: &'a dyn TextMeasure,
}

impl Layouter<'_> {
    /// Lays out `element` as the root of its own layout, `available` wide, at (0, 0).
    /// Its margins are left to the caller.
    pub(crate) fn lay_out_root(
        &self,
        element: NodeId,
        style: &ComputedStyle,
        available: AvailableSpace,
    ) -> LayoutBox {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        let mut root_style = style.clone();
        root_style.margin = Default::default();
        root_style.margin_left_auto = false;
        root_style.margin_right_auto = false;
        let root = self.build(&mut tree, element, root_style);
        self.compute(&mut tree, root, available);
        self.extract(&tree, root, 0.0, 0.0)
    }

    /// Lays out the whole document: its root element in a viewport `width` wide.
    pub(crate) fn lay_out_document(
        &self,
        html: NodeId,
        style: ComputedStyle,
        width: f32,
    ) -> LayoutBox {
        let mut tree = TaffyTree::new();
        tree.disable_rounding();
        let root = self.build(&mut tree, html, style);
        // A viewport box around the root element, so the root's own margins apply.
        let viewport = tree
            .new_with_children(
                Style {
                    display: TaffyDisplay::Block,
                    size: Size {
                        width: taffy::prelude::Dimension::length(width),
                        height: taffy::prelude::Dimension::auto(),
                    },
                    ..Style::default()
                },
                &[root],
            )
            .expect("a fresh taffy tree accepts a node");
        self.compute(&mut tree, viewport, AvailableSpace::Definite(width));
        self.extract(&tree, root, 0.0, 0.0)
    }

    /// Lays out an inline-level box shrink-to-fit in `available` width, at (0,
    /// 0), with its baseline set. Its margins are left to the line.
    pub(crate) fn lay_out_inline_block(
        &self,
        element: NodeId,
        style: &ComputedStyle,
        available: f32,
    ) -> LayoutBox {
        let mut block = style.clone();
        if matches!(block.display, Display::Inline | Display::InlineBlock) {
            block.display = Display::Block;
        }
        let space = |w: f32| {
            if w.is_finite() {
                AvailableSpace::Definite(w)
            } else {
                AvailableSpace::MaxContent
            }
        };
        let mut laid = if style.width.is_auto() {
            // Shrink to fit: as wide as the content wants, but no wider than the line.
            let preferred = self
                .lay_out_root(element, &block, AvailableSpace::MaxContent)
                .frame
                .w;
            self.lay_out_root(element, &block, space(preferred.min(available)))
        } else {
            self.lay_out_root(element, &block, space(available))
        };
        let replaced = replaced_size(self.document, element, style).is_some();
        laid.baseline = match laid.first_baseline() {
            Some(baseline) if !replaced => baseline,
            _ => laid.frame.h,
        };
        laid
    }

    fn compute(&self, tree: &mut TaffyTree<Context>, root: TaffyId, width: AvailableSpace) {
        let available = Size {
            width,
            height: AvailableSpace::MaxContent,
        };
        tree.compute_layout_with_measure(root, available, |inputs, _, context, style| {
            compute_leaf_layout(
                inputs,
                style,
                |_, _| 0.0,
                |known, space| match context {
                    Some(Context::Inline(run, _)) => {
                        let width = known.width.unwrap_or(match space.width {
                            AvailableSpace::Definite(w) => w,
                            AvailableSpace::MinContent => 0.0,
                            AvailableSpace::MaxContent => f32::INFINITY,
                        });
                        let laid = run.lay_out(self, width);
                        Size {
                            width: known.width.unwrap_or(laid.content_width),
                            height: laid.height,
                        }
                    }
                    _ => Size::ZERO,
                },
            )
        })
        .expect("layout of a well-formed taffy tree succeeds");
    }

    /// Adds `element` and its content to the tree.
    fn build(
        &self,
        tree: &mut TaffyTree<Context>,
        element: NodeId,
        style: ComputedStyle,
    ) -> TaffyId {
        let display = match style.display {
            Display::Flex => TaffyDisplay::Flex,
            _ => TaffyDisplay::Block,
        };
        let mut taffy = taffy_style(&style, display);

        if let Some((width, height)) = replaced_size(self.document, element, &style) {
            taffy.size = Size {
                width: taffy::prelude::Dimension::length(width),
                height: taffy::prelude::Dimension::length(height),
            };
            return new_node(tree, taffy, &[], Context::Element(element, style));
        }

        let children = if style.display == Display::Flex {
            self.flex_items(tree, element, &style)
        } else {
            self.block_children(tree, element, &style)
        };
        new_node(tree, taffy, &children, Context::Element(element, style))
    }

    /// A block's children: block-level elements as nodes, and each run of
    /// inline content between them as one inline leaf.
    fn block_children(
        &self,
        tree: &mut TaffyTree<Context>,
        element: NodeId,
        style: &ComputedStyle,
    ) -> Vec<TaffyId> {
        let mut children = Vec::new();
        let mut run = Collector::new(self.document, self.sheets);
        for &child in self.document[element].children() {
            let NodeData::Element(_) = self.document[child].data() else {
                run.add(child, style);
                continue;
            };
            let child_style = compute_style(self.document, child, style, self.sheets);
            match child_style.display {
                Display::None => {}
                Display::Block | Display::Flex => {
                    self.flush(tree, &mut run, style, &mut children);
                    children.push(self.build(tree, child, child_style));
                }
                Display::Inline | Display::InlineBlock => run.add_element(child, child_style),
            }
        }
        self.flush(tree, &mut run, style, &mut children);
        children
    }

    /// A flex container's items: each element, blockified, and each run of
    /// text directly inside (which the C++ engine ignored).
    fn flex_items(
        &self,
        tree: &mut TaffyTree<Context>,
        element: NodeId,
        style: &ComputedStyle,
    ) -> Vec<TaffyId> {
        let mut items = Vec::new();
        let mut run = Collector::new(self.document, self.sheets);
        for &child in self.document[element].children() {
            let NodeData::Element(_) = self.document[child].data() else {
                run.add(child, style);
                continue;
            };
            let mut child_style = compute_style(self.document, child, style, self.sheets);
            match child_style.display {
                Display::None => {}
                display => {
                    self.flush(tree, &mut run, style, &mut items);
                    if matches!(display, Display::Inline | Display::InlineBlock) {
                        child_style.display = Display::Block;
                    }
                    items.push(self.build(tree, child, child_style));
                }
            }
        }
        self.flush(tree, &mut run, style, &mut items);
        items
    }

    /// Ends the current inline run, adding it as a leaf if it has any content.
    fn flush<'d>(
        &self,
        tree: &mut TaffyTree<Context>,
        run: &mut Collector<'d>,
        parent: &ComputedStyle,
        children: &mut Vec<TaffyId>,
    ) {
        let finished = std::mem::replace(run, Collector::new(run.document, run.sheets));
        if let Some(inline) = finished.finish(parent.text_align) {
            let context = Context::Inline(inline, anonymous_style(parent));
            children.push(new_node(tree, anonymous_block(), &[], context));
        }
    }

    /// Reads a laid-out node back, at the parent's position (`x`, `y`).
    fn extract(&self, tree: &TaffyTree<Context>, node: TaffyId, x: f32, y: f32) -> LayoutBox {
        let layout = tree.layout(node).expect("the node is in the tree");
        let frame = Rect::new(
            x + layout.location.x,
            y + layout.location.y,
            layout.size.width,
            layout.size.height,
        );
        match tree.get_node_context(node) {
            Some(Context::Inline(run, style)) => {
                let mut laid = run.lay_out(self, frame.w);
                place(&mut laid, frame);
                LayoutBox {
                    node: None,
                    style: style.clone(),
                    frame,
                    baseline: 0.0,
                    children: laid.boxes,
                    lines: laid.lines,
                }
            }
            Some(Context::Element(element, style)) => LayoutBox {
                node: Some(*element),
                style: style.clone(),
                frame,
                baseline: 0.0,
                children: tree
                    .children(node)
                    .expect("the node is in the tree")
                    .into_iter()
                    .map(|child| self.extract(tree, child, frame.x, frame.y))
                    .collect(),
                lines: Vec::new(),
            },
            None => unreachable!("every node the layouter builds has a context"),
        }
    }
}

fn new_node(
    tree: &mut TaffyTree<Context>,
    style: Style,
    children: &[TaffyId],
    context: Context,
) -> TaffyId {
    let node = if children.is_empty() {
        tree.new_leaf_with_context(style, context)
    } else {
        tree.new_with_children(style, children).and_then(|node| {
            tree.set_node_context(node, Some(context))?;
            Ok(node)
        })
    };
    node.expect("a fresh taffy tree accepts a node")
}
