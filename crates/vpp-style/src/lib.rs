//! CSS.
//!
//! Parses stylesheets (tokenising with `cssparser`), encodes them as
//! `style.bin`, matches selectors, runs the cascade, and produces a computed
//! style for every element.
//!
//! # Where it sits
//!
//! Between the DOM and layout. The compiler uses it to produce `style.bin`; the
//! viewer uses it before every layout.
//!
//! # Not in this crate
//!
//! Box sizes and positions. Those are `vpp-layout`'s job.
//!
//! # Where to start reading
//!
//! [`StyleSheet`] (`stylesheet.rs`) and [`Selector`] (`selector.rs`), then
//! `parse.rs` for how CSS text becomes them and `binary.rs` for `style.bin`
//! (`docs/formats/style.md`). [`compute_style`] in `cascade.rs` turns them into
//! a [`ComputedStyle`] (`computed.rs`), using the defaults in `user_agent.rs`
//! and the property table in `properties.rs`, which reads values with
//! `values/`. The supported subset is listed in `docs/reference/runtime.md`,
//! "Supported CSS".
//!
mod binary;
mod cascade;
mod computed;
mod parse;
mod properties;
mod replaced;
mod selector;
mod stylesheet;
mod user_agent;
pub mod values;

pub use binary::{StyleBinError, decode_stylesheet, encode_stylesheet};
pub use cascade::compute_style;
pub use computed::{AlignItems, ComputedStyle, Display, FlexDirection, JustifyContent, TextAlign};
pub use parse::{parse_declarations, parse_stylesheet};
pub use replaced::replaced_size;
pub use selector::{Combinator, CompoundSelector, Selector};
pub use stylesheet::{Declaration, Rule, StyleSheet};
pub use values::color::Color;
pub use values::edges::Edges;
pub use values::length::Length;
