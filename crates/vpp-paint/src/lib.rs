//! Painting.
//!
//! Loads fonts and measures text for layout; builds a display list from
//! laid-out boxes and rasterises it into a pixel buffer with `tiny-skia`, text
//! with `swash`, and SVG with `resvg`. It also backs the canvas API.
//!
//! # Where it sits
//!
//! The last stage before the window. The display list is the seam where a
//! GPU backend could go later.
//!
//! # Not in this crate
//!
//! Windows and input. The viewer owns the window and hands this crate a
//! buffer to draw into. Finding the system's fonts is `vpp-platform`'s job.
//!
//! # Where to start reading
//!
//! [`render_page`] in `raster.rs`, which builds the display list
//! (`display_list.rs`) and draws it: shapes with `tiny-skia`, text with
//! `text.rs`, SVG with `svg.rs`. `font.rs` is [`FontSet`], which layout also
//! measures text with.
//!
//! # Status
//!
//! Ported (`docs/rust-port.md` §8, steps 5b and 5c). The C++ `canvas.cpp`, a
//! plain pixel buffer, is replaced by `tiny-skia`'s `Pixmap`.

mod display_list;
mod font;
mod raster;
mod svg;
mod text;

pub use display_list::{DisplayItem, build_display_list};
pub use font::{Font, FontError, FontSet};
pub use raster::{rasterize, render_page};
pub use resvg::tiny_skia::{Pixmap, PixmapMut};
