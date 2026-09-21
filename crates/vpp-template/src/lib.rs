//! Build-time templates.
//!
//! Expands layouts, includes, slots, and components (`docs/template-syntax.md`)
//! into one plain HTML page, and collects the page's stylesheets and scripts.
//!
//! # Where it sits
//!
//! Used only by the compiler. The viewer never sees a template
//! (`ARCHITECTURE.md`, invariant 8).
//!
//! # Not in this crate
//!
//! Runtime templating (`{{ }}` bindings, events, component JavaScript). That
//! needs its own design document first. Compiling the result into `dom.bin`,
//! `style.bin`, and `code.bin` is the compiler's job.
//!
//! # Where to start reading
//!
//! [`expand_page`] in `page.rs`, which applies `layout.rs`, then walks the
//! document with `expander.rs`, which calls `include.rs` and `component.rs`.
//! `slot.rs` moves content into slots, `substitute.rs` fills in `{{ }}`,
//! `scoped_css.rs` scopes component styles, and `preprocess.rs` fixes the HTML
//! text before parsing. The rules are listed in `docs/reference/compiler.md`,
//! "Templates".
//!
//! # Differences from the C++ expander
//!
//! Ordinary element nesting no longer counts towards the include limit; an
//! include at the top of an included file is expanded; and `data:` URLs and
//! `#fragment` references are left as written.

mod component;
mod error;
mod expander;
mod include;
mod layout;
mod page;
mod preprocess;
mod project;
mod props;
mod scoped_css;
mod slot;
mod substitute;
#[cfg(test)]
mod test_dir;

pub use error::TemplateError;
pub use page::{ExpandedPage, PageScript, PageStyle, expand_page};
pub use preprocess::preprocess_template_html;
pub use project::{TemplateOptions, find_project_root};
