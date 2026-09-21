//! A running page: its document, shared with its scripts, and its layout.

use std::cell::RefCell;
use std::rc::Rc;

use vpp_dom::{Document, NodeId};
use vpp_layout::{LayoutBox, layout_document};
use vpp_paint::{DisplayItem, FontSet, build_display_list};
use vpp_script::{HostRequest, QuickJsEngine, ScriptEngine};
use vpp_style::{Color, StyleSheet};

use crate::page_loader::LoadedPage;

/// A page with its scripts started.
pub(crate) struct Page {
    document: Rc<RefCell<Document>>,
    sheets: Vec<StyleSheet>,
    /// `None` only if the engine could not start, which the log says.
    engine: Option<QuickJsEngine>,
    layout: Option<LayoutBox>,
    /// The element the pointer went down on: a click needs down and up on the same one.
    pressed: Option<NodeId>,
    pub(crate) title: String,
    pub(crate) site_id: String,
    pub(crate) window: String,
}

impl Page {
    /// Starts `loaded`: runs its scripts in order. A script that fails is
    /// logged, and the ones after it still run.
    pub(crate) fn start(loaded: LoadedPage, log: &mut Vec<String>) -> Self {
        let document = Rc::new(RefCell::new(loaded.document));
        let mut engine = match QuickJsEngine::new(document.clone()) {
            Ok(engine) => Some(engine),
            Err(e) => {
                log.push(format!("script engine did not start: {e}"));
                None
            }
        };
        if let Some(engine) = &mut engine {
            for script in &loaded.scripts {
                match engine.run(script) {
                    Ok(()) => log.push(format!("script: {}", script.filename)),
                    Err(e) => log.push(format!("script error: {e}")),
                }
            }
        }
        Self {
            document,
            sheets: loaded.sheets,
            engine,
            layout: None,
            pressed: None,
            title: loaded.title,
            site_id: loaded.site_id,
            window: loaded.window,
        }
    }

    /// Lays the page out `viewport_width` CSS pixels wide, `top` CSS pixels
    /// down the window (below the shell).
    pub(crate) fn lay_out(&mut self, fonts: &FontSet, viewport_width: f32, top: f32) {
        self.layout = layout_document(&self.document.borrow(), &self.sheets, fonts, viewport_width);
        if let Some(root) = &mut self.layout {
            root.translate(0.0, top);
        }
    }

    /// The height of the laid-out page in CSS pixels, from its top edge.
    pub(crate) fn height(&self) -> f32 {
        self.layout.as_ref().map_or(0.0, |root| root.frame.h)
    }

    /// Whether a button or link is under a point in CSS pixels: the page's
    /// top strip drags a frameless window everywhere else.
    pub(crate) fn is_control_at(&self, x: f32, y: f32) -> bool {
        let Some(node) = self.element_at(x, y) else {
            return false;
        };
        let document = self.document.borrow();
        document.contains(node)
            && (document.closest(node, "button").is_some() || document.closest(node, "a").is_some())
    }

    /// The element under a point in CSS pixels.
    fn element_at(&self, x: f32, y: f32) -> Option<NodeId> {
        self.layout.as_ref()?.hit_test(x, y)?.node
    }

    /// The pointer went down at a point in CSS pixels.
    pub(crate) fn press(&mut self, x: f32, y: f32) {
        self.pressed = self.element_at(x, y);
    }

    /// The pointer went up at a point in CSS pixels. If that completes a
    /// click, dispatches it to the scripts and returns the `href` of the link
    /// clicked, if any.
    pub(crate) fn release(&mut self, x: f32, y: f32) -> Option<String> {
        let pressed = self.pressed.take();
        let node = self.element_at(x, y).filter(|&n| Some(n) == pressed)?;
        // Read the link before scripts run: a listener may change the page.
        let href = {
            let document = self.document.borrow();
            if !document.contains(node) {
                return None;
            }
            document
                .closest(node, "a")
                .and_then(|a| document.element(a)?.attribute("href"))
                .map(str::to_owned)
        };
        if let Some(engine) = &mut self.engine {
            engine.dispatch_click(node);
        }
        href
    }

    /// What the scripts asked the viewer to do since last asked.
    pub(crate) fn take_requests(&mut self) -> Vec<HostRequest> {
        self.engine
            .as_mut()
            .map(ScriptEngine::take_requests)
            .unwrap_or_default()
    }

    /// The canvas colour: the background of `<html>`, else of `<body>`, as in a browser.
    pub(crate) fn background(&self) -> Option<Color> {
        let root = self.layout.as_ref()?;
        if root.style.has_background() {
            return Some(root.style.background);
        }
        let document = self.document.borrow();
        root.children
            .iter()
            .find(|b| b.node.is_some_and(|n| document[n].is_element("body")))
            .filter(|body| body.style.has_background())
            .map(|body| body.style.background)
    }

    /// Everything the page draws, in CSS pixels.
    pub(crate) fn display_list(&self) -> Vec<DisplayItem> {
        match &self.layout {
            Some(root) => build_display_list(root, &self.document.borrow()),
            None => Vec::new(),
        }
    }

    /// The border box of the element `pick` finds, for tests to click on.
    #[cfg(test)]
    pub(crate) fn element_box(
        &self,
        pick: impl FnOnce(&Document) -> Option<NodeId>,
    ) -> Option<vpp_layout::Rect> {
        fn find(b: &LayoutBox, node: NodeId) -> Option<vpp_layout::Rect> {
            if b.node == Some(node) {
                return Some(b.frame);
            }
            b.children.iter().find_map(|c| find(c, node))
        }
        let node = pick(&self.document.borrow())?;
        find(self.layout.as_ref()?, node)
    }

    /// The text of the element with `id`, for tests.
    #[cfg(test)]
    pub(crate) fn text_of(&self, id: &str) -> Option<String> {
        let document = self.document.borrow();
        let node = document.find_by_id(document.root(), id)?;
        Some(document.text_content(node))
    }
}
