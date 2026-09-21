//! Painting.
//!
//! Builds a display list from laid-out boxes and rasterises it into a pixel buffer with `tiny-skia`, text with `swash`, and SVG with `resvg`. It also backs the canvas API.
//!
//! # Where it sits
//!
//! The last stage before the window. The display list is the seam where a GPU backend could go later.
//!
//! # Not in this crate
//!
//! Windows and input. The viewer owns the window and hands this crate a buffer to draw into.
//!
//! # Status
//!
//! Not ported yet. The C++ reference implementation is `runtime/src/paint.cpp`, `font.cpp`, `svg.cpp`, and `canvas.cpp`. The planned Rust files are listed in `docs/rust-port.md` §5.3.
