//! Includes: `<vpp-include src="/includes/footer.html">` pastes a fragment.
//! The tag's other attributes become `{{ variables }}` inside it, and
//! `optional` makes a missing file expand to nothing.

use vpp_dom::{Document, NodeId};

use crate::error::TemplateError;
use crate::expander::Expander;
use crate::props::properties;
use crate::substitute::substitute;

impl Expander<'_> {
    /// The expanded content for the `<vpp-include>` element `tag`, detached.
    pub(crate) fn expand_include(
        &mut self,
        document: &mut Document,
        tag: NodeId,
        nesting: usize,
    ) -> Result<Vec<NodeId>, TemplateError> {
        let element = document.element(tag).expect("an include tag is an element");
        let Some(src) = element.attribute("src").map(str::to_owned) else {
            return Err(TemplateError::new(
                &self.root(),
                "<vpp-include> needs a src attribute",
            ));
        };
        let optional = element.attribute("optional").is_some();
        let vars = properties(element, &["src", "optional"]);

        let file = self.resolve_from_root(&src)?;
        if !file.is_file() {
            if optional {
                return Ok(Vec::new());
            }
            return Err(TemplateError::new(
                &self.root(),
                format!("include not found: {src}"),
            ));
        }
        let (fragment, nodes) = self.load_fragment(&file)?;
        let imported = nodes
            .into_iter()
            .map(|n| document.import(&fragment, n))
            .collect();
        let holder = Self::hold(document, imported);
        substitute(document, holder, &vars);
        self.expand_children(document, holder, nesting + 1)?;
        Ok(Self::release(document, holder))
    }
}
