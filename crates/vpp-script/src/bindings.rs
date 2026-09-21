//! The native interface under `prelude.js`: a handful of functions that take
//! and return plain strings and numbers.
//!
//! Elements cross into JavaScript as numeric handles. Every handle is checked
//! here: one that was never given out, or whose element has been removed,
//! throws a `TypeError` instead of reaching anything.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rquickjs::{Ctx, Exception, Function, Object, Result};
use vpp_dom::{Document, NodeId};

use crate::engine::HostRequest;

/// What the natives share with the engine.
pub(crate) struct Shared {
    pub(crate) document: Rc<RefCell<Document>>,
    pub(crate) handles: RefCell<Handles>,
    pub(crate) requests: RefCell<Vec<HostRequest>>,
}

/// The elements given to JavaScript, by handle.
#[derive(Default)]
pub(crate) struct Handles {
    nodes: Vec<NodeId>,
    index: HashMap<NodeId, u32>,
}

impl Handles {
    /// The handle for `node`, giving out a new one the first time.
    pub(crate) fn handle(&mut self, node: NodeId) -> u32 {
        *self.index.entry(node).or_insert_with(|| {
            self.nodes.push(node);
            (self.nodes.len() - 1) as u32
        })
    }

    fn node(&self, handle: u32) -> Option<NodeId> {
        self.nodes.get(handle as usize).copied()
    }
}

impl Shared {
    fn request(&self, request: HostRequest) {
        let mut requests = self.requests.borrow_mut();
        // One pending re-layout is enough, however many changes caused it.
        if request == HostRequest::Invalidate && requests.contains(&HostRequest::Invalidate) {
            return;
        }
        requests.push(request);
    }

    /// The element behind `handle`, if it was given out and is still in the document.
    fn element<'js>(&self, ctx: &Ctx<'js>, handle: u32) -> Result<NodeId> {
        let node = self.handles.borrow().node(handle);
        // `get`, not `element`: the node may be gone, and ids are never reused.
        let alive = |node| {
            self.document
                .borrow()
                .get(node)
                .is_some_and(|n| n.as_element().is_some())
        };
        match node {
            Some(node) if alive(node) => Ok(node),
            Some(_) => Err(Exception::throw_type(
                ctx,
                "the element was removed from the page",
            )),
            None => Err(Exception::throw_type(ctx, "not an element of this page")),
        }
    }
}

/// The native object `prelude.js` is called with.
pub(crate) fn natives<'js>(ctx: &Ctx<'js>, shared: &Rc<Shared>) -> Result<Object<'js>> {
    let native = Object::new(ctx.clone())?;

    let s = shared.clone();
    native.set(
        "byId",
        Function::new(ctx.clone(), move |id: String| -> Option<u32> {
            let found = {
                let document = s.document.borrow();
                document.find_by_id(document.root(), &id)
            };
            found.map(|node| s.handles.borrow_mut().handle(node))
        })?,
    )?;

    let s = shared.clone();
    native.set(
        "id",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, handle: u32| -> Result<String> {
                let node = s.element(&ctx, handle)?;
                let document = s.document.borrow();
                Ok(document
                    .element(node)
                    .and_then(|e| e.id())
                    .unwrap_or_default()
                    .to_owned())
            },
        )?,
    )?;

    let s = shared.clone();
    native.set(
        "tagName",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, handle: u32| -> Result<String> {
                let node = s.element(&ctx, handle)?;
                let document = s.document.borrow();
                Ok(document
                    .element(node)
                    .map(|e| e.tag.to_ascii_uppercase())
                    .unwrap_or_default())
            },
        )?,
    )?;

    let s = shared.clone();
    native.set(
        "getText",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, handle: u32| -> Result<String> {
                let node = s.element(&ctx, handle)?;
                Ok(s.document.borrow().text_content(node))
            },
        )?,
    )?;

    let s = shared.clone();
    native.set(
        "setText",
        Function::new(
            ctx.clone(),
            move |ctx: Ctx<'js>, handle: u32, text: String| -> Result<()> {
                let node = s.element(&ctx, handle)?;
                s.document.borrow_mut().set_text_content(node, text);
                s.request(HostRequest::Invalidate);
                Ok(())
            },
        )?,
    )?;

    let s = shared.clone();
    native.set(
        "log",
        Function::new(ctx.clone(), move |line: String| {
            s.request(HostRequest::Log(line))
        })?,
    )?;
    let s = shared.clone();
    native.set(
        "popup",
        Function::new(ctx.clone(), move |message: String| {
            s.request(HostRequest::Popup(message))
        })?,
    )?;
    let s = shared.clone();
    native.set(
        "close",
        Function::new(ctx.clone(), move || s.request(HostRequest::Close))?,
    )?;
    let s = shared.clone();
    native.set(
        "minimize",
        Function::new(ctx.clone(), move || s.request(HostRequest::Minimize))?,
    )?;
    let s = shared.clone();
    native.set(
        "maximize",
        Function::new(ctx.clone(), move || s.request(HostRequest::Maximize))?,
    )?;

    Ok(native)
}
