#include "vpp/paint.h"

#include <algorithm>
#include <cmath>

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

void paintBox(const LayoutBox& box, Canvas& canvas, const FontSet& fonts, float scale) {
    paintBackgroundAndBorder(box, canvas, scale);

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
