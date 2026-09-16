#pragma once

#include "vpp/css.h"
#include "vpp/dom.h"
#include "vpp/font.h"
#include "vpp/style.h"

#include <memory>
#include <string>
#include <vector>

namespace vpp {

struct RectF {
    float x = 0;
    float y = 0;
    float w = 0;
    float h = 0;

    bool contains(float px, float py) const {
        return px >= x && py >= y && px < x + w && py < y + h;
    }
};

struct LayoutBox;

// One piece of a line: a run of text, or an inline-level box positioned in the line.
struct Fragment {
    RectF rect;             // absolute, CSS px
    float baseline = 0;     // distance from rect.y to the text baseline
    std::string text;       // empty when this fragment is a box
    Color color;
    float fontSize = 16;
    bool bold = false;
    const LayoutBox* box = nullptr;
};

struct Line {
    RectF rect;
    std::vector<Fragment> fragments;
};

// The layout tree. Separate from the DOM: a box may have no node (anonymous
// containers for inline content), and a node with display:none has no box.
struct LayoutBox {
    const Node* node = nullptr;
    ComputedStyle style;
    RectF frame;            // border box, absolute, CSS px
    float baseline = 0;     // for inline-level boxes: distance from frame.y to the first baseline
    std::vector<std::unique_ptr<LayoutBox>> children;
    std::vector<Line> lines; // present when this box directly contains inline content
};

struct LayoutContext {
    const FontSet& fonts;
    float scale = 1.0f;               // device pixels per CSS px, used for text measurement
    const StyleSheet* sheet = nullptr; // author stylesheet, may be null
};

// Lays out a document at the given viewport width. Returns null if the
// document has no root element.
std::unique_ptr<LayoutBox> layoutDocument(const Node& document, const LayoutContext& ctx,
                                          float viewportWidth);

// Deepest box with a DOM node under (x, y), or null.
const LayoutBox* hitTest(const LayoutBox& root, float x, float y);

// Moves a laid-out tree by (dx, dy), for stacking documents in one window.
void translateLayout(LayoutBox& root, float dx, float dy);

} // namespace vpp
