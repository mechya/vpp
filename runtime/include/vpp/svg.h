#pragma once

#include "vpp/canvas.h"

#include <string>

namespace vpp {

// Maps path coordinates to device pixels: device = point * scale + offset.
struct PathTransform {
    float scale = 1.0f;
    float tx = 0.0f;
    float ty = 0.0f;
};

// Fills an SVG path ("d" attribute syntax: M L H V C S Q T A Z, absolute and
// relative) with antialiasing. Curves and arcs are flattened; nonzero or
// even-odd winding.
void fillSvgPath(Canvas& canvas, const std::string& d, const PathTransform& t, Color color, bool evenOdd);

} // namespace vpp
