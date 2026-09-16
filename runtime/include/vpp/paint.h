#pragma once

#include "vpp/canvas.h"
#include "vpp/font.h"
#include "vpp/layout.h"

namespace vpp {

// Draws a layout tree onto the canvas. scale converts CSS px to device px.
void paint(const LayoutBox& root, Canvas& canvas, const FontSet& fonts, float scale);

} // namespace vpp
