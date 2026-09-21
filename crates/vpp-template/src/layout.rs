//! Layouts: `<vpp-layout src="/layouts/main.html">` at the top of a page's body
//! wraps the page in that layout. The page's `<vpp-fill>` content goes into
//! the layout's slots, and the layout becomes the document. Layouts can use
//! layouts.

use vpp_dom::{Document, NodeId};

use crate::error::TemplateError;
use crate::expander::Expander;
use crate::slot::Fills;

/// The `<vpp-layout>` element directly inside `<body>`, if there is one.
pub(crate) fn find_layout_tag(document: &Document) -> Option<NodeId> {
    let body = document.find_first(document.root(), "body")?;
    document[body]
        .children()
        .iter()
        .copied()
        .find(|&c| document[c].is_element("vpp-layout"))
}

impl Expander<'_> {
    /// Loads the layout named by `layout_tag`, fills its slots with the
    /// tag's content, and returns it as the new document.
    pub(crate) fn apply_layout(
        &self,
        mut page: Document,
        layout_tag: NodeId,
    ) -> Result<Document, TemplateError> {
        let Some(src) = page
            .element(layout_tag)
            .and_then(|e| e.attribute("src"))
            .map(str::to_owned)
        else {
            return Err(TemplateError::new(
                &self.root(),
                "<vpp-layout> needs a src attribute",
            ));
        };
        let file = self.resolve_from_root(&src)?;
        let mut layout = self.load_document(&file)?;
        let fills = Fills::take_from(&mut page, layout_tag);
        let root = layout.root();
        fills.fill_slots(&mut layout, root);
        Ok(layout)
    }
}
