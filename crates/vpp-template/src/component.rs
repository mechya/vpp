//! Components: `<vpp-component src="/components/stat-card" value="3">`, or a
//! custom tag such as `<stat-card>`, expands `component.html` from that
//! directory. The tag's attributes become `{{ properties }}`, its children
//! fill the template's slots, and `component.css` is scoped to the component
//! (`scoped_css.rs`). Components can contain components and includes.

use vpp_dom::{Document, NodeId};

use crate::error::TemplateError;
use crate::expander::Expander;
use crate::page::PageStyle;
use crate::props::properties;
use crate::scoped_css::{scope_class, scope_stylesheet, wants_scoping};
use crate::slot::Fills;
use crate::substitute::substitute;

impl Expander<'_> {
    /// The expanded content for component tag `tag`, whose directory is `src`, detached.
    pub(crate) fn expand_component(
        &mut self,
        document: &mut Document,
        tag: NodeId,
        src: &str,
        nesting: usize,
    ) -> Result<Vec<NodeId>, TemplateError> {
        let dir = self.resolve_from_root(src)?;
        let html_file = dir.join("component.html");
        if !html_file.is_file() {
            return Err(TemplateError::new(
                &self.root(),
                format!("component not found: {src}"),
            ));
        }
        let (fragment, nodes) = self.load_fragment(&html_file)?;
        let imported = nodes
            .into_iter()
            .map(|n| document.import(&fragment, n))
            .collect();
        let holder = Self::hold(document, imported);

        let key = self.project.project_path(&dir);
        let css_file = dir.join("component.css");
        let has_css = css_file.is_file();
        let scoped = has_css && wants_scoping(&dir);
        let scope = scope_class(&key);

        // The scope class goes on the template's own elements, before slots
        // bring in the page's content, which stays unscoped.
        if scoped {
            let elements: Vec<_> = document.descendants(holder).skip(1).collect();
            for node in elements {
                if let Some(element) = document.element_mut(node) {
                    let class = match element.attribute("class") {
                        Some(existing) if !existing.is_empty() => format!("{existing} {scope}"),
                        _ => scope.clone(),
                    };
                    element.set_attribute("class", class);
                }
            }
        }
        if has_css && self.styled_components.insert(key.clone()) {
            let css = std::fs::read_to_string(&css_file)
                .map_err(|e| TemplateError::new(&css_file, format!("cannot read file: {e}")))?;
            let mut sheet = vpp_style::parse_stylesheet(&css);
            if scoped {
                scope_stylesheet(&mut sheet, &scope);
            }
            let dir_name = dir
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            self.component_styles.push(PageStyle {
                name: format!("component-{}", crate::page::sanitize_name(&dir_name)),
                path: Some(css_file),
                sheet,
            });
        }
        if dir.join("component.js").is_file() {
            self.warnings.push(format!(
                "{key}/component.js ignored: component JavaScript is not supported yet"
            ));
        }

        let props = properties(
            document
                .element(tag)
                .expect("a component tag is an element"),
            &["src"],
        );
        substitute(document, holder, &props);
        let fills = Fills::take_from(document, tag);
        fills.fill_slots(document, holder);
        self.expand_children(document, holder, nesting + 1)?;
        Ok(Self::release(document, holder))
    }
}
