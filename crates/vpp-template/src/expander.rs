//! The expander: loads template files, and walks a document replacing
//! `<vpp-include>`, `<vpp-component>`, and custom component tags with their
//! content. Layouts (`layout.rs`) are applied before this walk.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use vpp_dom::{Document, NodeId};

use crate::error::TemplateError;
use crate::page::PageStyle;
use crate::preprocess::preprocess_template_html;
use crate::project::{Project, TemplateOptions};

/// How deeply includes and components may nest inside each other, and layouts
/// wrap each other. Stops a template that includes itself.
pub(crate) const MAX_NESTING: usize = 32;

pub(crate) struct Expander<'a> {
    pub(crate) options: &'a TemplateOptions,
    pub(crate) project: Project,
    pub(crate) warnings: Vec<String>,
    /// Component stylesheets, each added once, after the page's own stylesheets.
    pub(crate) component_styles: Vec<PageStyle>,
    pub(crate) styled_components: HashSet<String>,
}

impl<'a> Expander<'a> {
    pub(crate) fn new(options: &'a TemplateOptions) -> Result<Self, TemplateError> {
        Ok(Self {
            options,
            project: Project::new(&options.project_root)?,
            warnings: Vec::new(),
            component_styles: Vec::new(),
            styled_components: HashSet::new(),
        })
    }

    /// Errors about the project as a whole are reported against its root.
    pub(crate) fn root(&self) -> PathBuf {
        self.project.root.clone()
    }

    /// Resolves a project-absolute reference, as the expander rewrites them all to be.
    pub(crate) fn resolve_from_root(&self, reference: &str) -> Result<PathBuf, TemplateError> {
        let root = self.root();
        self.project.resolve(reference, &root, &root)
    }

    /// Reads, pre-processes, and parses a template file, then rewrites its
    /// references to project-absolute paths, so its nodes can move into other
    /// files without losing where they point.
    pub(crate) fn load_document(&self, file: &Path) -> Result<Document, TemplateError> {
        let html = std::fs::read_to_string(file)
            .map_err(|e| TemplateError::new(file, format!("cannot read file: {e}")))?;
        let mut document = vpp_dom::parse_html(&preprocess_template_html(&html));
        self.rewrite_references(&mut document, file)?;
        Ok(document)
    }

    /// A file's `<head>` children followed by its `<body>` children, detached,
    /// in the returned document.
    pub(crate) fn load_fragment(
        &self,
        file: &Path,
    ) -> Result<(Document, Vec<NodeId>), TemplateError> {
        let mut document = self.load_document(file)?;
        let mut nodes = Vec::new();
        for tag in ["head", "body"] {
            if let Some(section) = document.find_first(document.root(), tag) {
                nodes.extend(document.take_children(section));
            }
        }
        Ok((document, nodes))
    }

    fn rewrite_references(
        &self,
        document: &mut Document,
        file: &Path,
    ) -> Result<(), TemplateError> {
        let base = file.parent().unwrap_or(&self.project.root).to_path_buf();
        let elements: Vec<_> = document.descendants(document.root()).collect();
        for node in elements {
            let Some(element) = document.element(node) else {
                continue;
            };
            // Links between pages (<a href>) stay as written.
            let attribute = match element.tag.as_str() {
                "link" => "href",
                "script" | "img" | "vpp-include" | "vpp-component" | "vpp-layout" => "src",
                _ => continue,
            };
            let Some(value) = element.attribute(attribute) else {
                continue;
            };
            if value.is_empty()
                || value.contains("://")
                || value.starts_with("data:")
                || value.starts_with('#')
            {
                continue;
            }
            let resolved = self.project.resolve(value, &base, file)?;
            let rewritten = self.project.project_path(&resolved);
            document
                .element_mut(node)
                .expect("is an element")
                .set_attribute(attribute, rewritten);
        }
        Ok(())
    }

    /// Expands every include and component below `parent`. `nesting` counts
    /// includes and components only, not ordinary element depth.
    pub(crate) fn expand_children(
        &mut self,
        document: &mut Document,
        parent: NodeId,
        nesting: usize,
    ) -> Result<(), TemplateError> {
        if nesting > MAX_NESTING {
            return Err(TemplateError::new(
                &self.root(),
                "includes or components nest too deeply; does one include itself?",
            ));
        }
        let mut i = 0;
        while let Some(&child) = document[parent].children().get(i) {
            let Some(tag) = document.element(child).map(|e| e.tag.clone()) else {
                i += 1;
                continue;
            };
            let replacement = match tag.as_str() {
                "vpp-include" => Some(self.expand_include(document, child, nesting)?),
                "vpp-component" => {
                    let src = document.element(child).and_then(|e| e.attribute("src"));
                    let Some(src) = src.map(str::to_owned) else {
                        return Err(TemplateError::new(
                            &self.root(),
                            "<vpp-component> needs a src attribute",
                        ));
                    };
                    Some(self.expand_component(document, child, &src, nesting)?)
                }
                _ => match self.custom_component(&tag) {
                    Some(dir) => Some(self.expand_component(document, child, &dir, nesting)?),
                    None => None,
                },
            };
            match replacement {
                Some(nodes) => {
                    document.remove(child);
                    for node in nodes {
                        document.insert_child(parent, i, node);
                        i += 1;
                    }
                }
                None => {
                    self.expand_children(document, child, nesting)?;
                    i += 1;
                }
            }
        }
        Ok(())
    }

    /// The component directory for a custom tag (one with a hyphen, not
    /// `vpp-`): from `vpp.json` aliases, or `components/<tag>/` if it exists.
    fn custom_component(&self, tag: &str) -> Option<String> {
        if !tag.contains('-') || tag.starts_with("vpp-") {
            return None;
        }
        if let Some(dir) = self.options.component_aliases.get(tag) {
            return Some(dir.clone());
        }
        let conventional = self.project.root.join("components").join(tag);
        conventional
            .join("component.html")
            .is_file()
            .then(|| format!("/components/{tag}"))
    }

    /// Puts `nodes` under a temporary holder element, so a list of top-level
    /// nodes can be expanded like any element's children.
    pub(crate) fn hold(document: &mut Document, nodes: Vec<NodeId>) -> NodeId {
        let holder = document.create_element("vpp-holder");
        for node in nodes {
            document.append_child(holder, node);
        }
        holder
    }

    /// Takes the expanded nodes back out of a holder and removes it.
    pub(crate) fn release(document: &mut Document, holder: NodeId) -> Vec<NodeId> {
        let nodes = document.take_children(holder);
        document.remove(holder);
        nodes
    }
}
