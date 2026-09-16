#include "vpp/paint.h"

#include "vpp/svg.h"

#include <algorithm>
#include <cmath>
#include <cstdlib>
#include <functional>
#include <sstream>

namespace vpp {

namespace {

int px(float v, float scale) {
    return static_cast<int>(std::lround(v * scale));
}

Rect toDevice(const RectF& r, float scale) {
    const int x0 = px(r.x, scale);
    const int y0 = px(r.y, scale);
    const int x1 = px(r.x + r.w, scale);
    const int y1 = px(r.y + r.h, scale);
    return {x0, y0, x1 - x0, y1 - y0};
}

void paintBackgroundAndBorder(const LayoutBox& box, Canvas& canvas, float scale) {
    const ComputedStyle& s = box.style;
    const Rect frame = toDevice(box.frame, scale);
    const int radius = px(s.borderRadius, scale);
    const int border = s.borderWidth > 0 ? std::max(px(s.borderWidth, scale), 1) : 0;

    if (border > 0 && s.hasBackground) {
        canvas.fillRoundRect(frame, radius, s.borderColor);
        const Rect inner{frame.x + border, frame.y + border, frame.w - 2 * border, frame.h - 2 * border};
        canvas.fillRoundRect(inner, std::max(radius - border, 0), s.background);
        return;
    }
    if (s.hasBackground) canvas.fillRoundRect(frame, radius, s.background);
    if (border > 0) {
        canvas.fillRect({frame.x, frame.y, frame.w, border}, s.borderColor);
        canvas.fillRect({frame.x, frame.y + frame.h - border, frame.w, border}, s.borderColor);
        canvas.fillRect({frame.x, frame.y, border, frame.h}, s.borderColor);
        canvas.fillRect({frame.x + frame.w - border, frame.y, border, frame.h}, s.borderColor);
    }
}

// Inline <svg>: every <path> descendant is filled. The viewBox is fitted into
// the content box, preserving aspect ratio, as SVG's default does.
void paintSvg(const LayoutBox& box, Canvas& canvas, float scale) {
    const Node& svg = *box.node;
    const ComputedStyle& s = box.style;
    const float inset = s.borderWidth;
    const RectF content{box.frame.x + s.padding.left + inset, box.frame.y + s.padding.top + inset,
                        box.frame.w - s.padding.left - s.padding.right - 2 * inset,
                        box.frame.h - s.padding.top - s.padding.bottom - 2 * inset};

    float vbX = 0, vbY = 0, vbW = content.w, vbH = content.h;
    if (const std::string* vb = svg.attribute("viewbox")) {
        std::istringstream in(*vb);
        float a, b, c, d;
        if (in >> a >> b >> c >> d && c > 0 && d > 0) {
            vbX = a;
            vbY = b;
            vbW = c;
            vbH = d;
        }
    }
    const float fit = std::min(content.w / vbW, content.h / vbH) * scale;
    PathTransform t;
    t.scale = fit;
    t.tx = (content.x + (content.w - vbW * fit / scale) / 2) * scale - vbX * fit;
    t.ty = (content.y + (content.h - vbH * fit / scale) / 2) * scale - vbY * fit;

    const std::string* svgFill = svg.attribute("fill");
    std::function<void(const Node&)> visit = [&](const Node& n) {
        if (n.isElement() && n.tag() == "path") {
            const std::string* d = n.attribute("d");
            const std::string* fill = n.attribute("fill") ? n.attribute("fill") : svgFill;
            if (!d) return;
            Color color = s.color;
            if (fill && *fill != "currentcolor" && *fill != "currentColor") {
                if (*fill == "none") return;
                parseCssColor(*fill, color);
            }
            const std::string* rule = n.attribute("fill-rule");
            fillSvgPath(canvas, *d, t, color, rule && *rule == "evenodd");
        }
        for (const auto& c : n.children()) visit(*c);
    };
    visit(svg);
}

void paintBox(const LayoutBox& box, Canvas& canvas, const FontSet& fonts, float scale) {
    paintBackgroundAndBorder(box, canvas, scale);

    if (box.node && box.node->tag() == "svg") {
        paintSvg(box, canvas, scale);
        return;
    }

    if (!box.lines.empty()) {
        for (const Line& line : box.lines) {
            for (const Fragment& f : line.fragments) {
                if (f.box) {
                    paintBox(*f.box, canvas, fonts, scale);
                    continue;
                }
                const Font& font = fonts.pick(f.bold);
                font.draw(canvas, px(f.rect.x, scale), px(f.rect.y + f.baseline, scale), f.text,
                          f.fontSize * scale, f.color);
            }
        }
        return;
    }

    for (const auto& child : box.children)
        paintBox(*child, canvas, fonts, scale);
}

} // namespace

void paint(const LayoutBox& root, Canvas& canvas, const FontSet& fonts, float scale) {
    paintBox(root, canvas, fonts, scale);
}

} // namespace vpp
