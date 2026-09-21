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
//! Not implemented yet. The removed C++ implementation was `runtime/src/paint.cpp`, `font.cpp`, `svg.cpp`, and `canvas.cpp`; read it with `git show 1d1cd11:<path>`, and its behaviour in `docs/reference/`. The planned Rust files are listed in `docs/rust-port.md` §5.3.
