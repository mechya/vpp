#include "vpp/canvas.h"

#include <algorithm>
#include <cmath>

namespace vpp {

namespace {

uint32_t pack(Color c) {
    return (static_cast<uint32_t>(c.a) << 24) | (static_cast<uint32_t>(c.r) << 16) |
           (static_cast<uint32_t>(c.g) << 8) | static_cast<uint32_t>(c.b);
}

} // namespace

Canvas::Canvas(int width, int height) : width_(0), height_(0) {
    resize(width, height);
}

void Canvas::resize(int width, int height) {
    width_ = std::max(width, 1);
    height_ = std::max(height, 1);
    pixels_.assign(static_cast<size_t>(width_) * height_, 0xFF000000u);
}

void Canvas::clear(Color c) {
    std::fill(pixels_.begin(), pixels_.end(), pack(c));
}

void Canvas::blendPixel(int x, int y, Color c, uint8_t coverage) {
    if (x < 0 || y < 0 || x >= width_ || y >= height_) return;

    const uint32_t alpha = (static_cast<uint32_t>(c.a) * coverage) / 255;
    if (alpha == 0) return;

    uint32_t& dst = pixels_[static_cast<size_t>(y) * width_ + x];
    if (alpha == 255) {
        dst = pack(c);
        return;
    }

    const uint32_t inv = 255 - alpha;
    const uint32_t dr = (dst >> 16) & 0xFF;
    const uint32_t dg = (dst >> 8) & 0xFF;
    const uint32_t db = dst & 0xFF;

    const uint32_t r = (c.r * alpha + dr * inv) / 255;
    const uint32_t g = (c.g * alpha + dg * inv) / 255;
    const uint32_t b = (c.b * alpha + db * inv) / 255;
    dst = 0xFF000000u | (r << 16) | (g << 8) | b;
}

void Canvas::fillRect(Rect r, Color c) {
    const int x0 = std::max(r.x, 0);
    const int y0 = std::max(r.y, 0);
    const int x1 = std::min(r.x + r.w, width_);
    const int y1 = std::min(r.y + r.h, height_);
    if (x0 >= x1 || y0 >= y1) return;

    if (c.a == 255) {
        const uint32_t v = pack(c);
        for (int y = y0; y < y1; ++y) {
            uint32_t* row = &pixels_[static_cast<size_t>(y) * width_];
            std::fill(row + x0, row + x1, v);
        }
        return;
    }

    for (int y = y0; y < y1; ++y)
        for (int x = x0; x < x1; ++x)
            blendPixel(x, y, c);
}

void Canvas::maskRoundedCorners(int radius) {
    radius = std::clamp(radius, 0, std::min(width_, height_) / 2);
    if (radius == 0) return;
    const float rad = static_cast<float>(radius);
    const float cxL = rad - 0.5f, cxR = static_cast<float>(width_ - radius) - 0.5f;
    const float cyT = rad - 0.5f, cyB = static_cast<float>(height_ - radius) - 0.5f;

    auto maskCorner = [&](int x0, int y0, float cx, float cy) {
        for (int y = y0; y < y0 + radius; ++y) {
            for (int x = x0; x < x0 + radius; ++x) {
                const float dist = std::sqrt((static_cast<float>(x) - cx) * (static_cast<float>(x) - cx) +
                                             (static_cast<float>(y) - cy) * (static_cast<float>(y) - cy));
                const float coverage = std::clamp(rad + 0.5f - dist, 0.0f, 1.0f);
                uint32_t& px = pixels_[static_cast<size_t>(y) * width_ + x];
                const uint32_t alpha = static_cast<uint32_t>(coverage * 255.0f + 0.5f);
                px = (px & 0x00FFFFFFu) | (alpha << 24);
            }
        }
    };
    maskCorner(0, 0, cxL, cyT);
    maskCorner(width_ - radius, 0, cxR, cyT);
    maskCorner(0, height_ - radius, cxL, cyB);
    maskCorner(width_ - radius, height_ - radius, cxR, cyB);
}

void Canvas::fillRoundRect(Rect r, int radius, Color c) {
    radius = std::clamp(radius, 0, std::min(r.w, r.h) / 2);
    if (radius == 0) {
        fillRect(r, c);
        return;
    }

    const int x0 = std::max(r.x, 0);
    const int y0 = std::max(r.y, 0);
    const int x1 = std::min(r.x + r.w, width_);
    const int y1 = std::min(r.y + r.h, height_);
    if (x0 >= x1 || y0 >= y1) return;

    // Corner circle centres in pixel-centre space.
    const float cxL = static_cast<float>(r.x + radius) - 0.5f;
    const float cxR = static_cast<float>(r.x + r.w - radius) - 0.5f;
    const float cyT = static_cast<float>(r.y + radius) - 0.5f;
    const float cyB = static_cast<float>(r.y + r.h - radius) - 0.5f;
    const float rad = static_cast<float>(radius);

    for (int y = y0; y < y1; ++y) {
        for (int x = x0; x < x1; ++x) {
            const float px = static_cast<float>(x);
            const float py = static_cast<float>(y);

            const bool inCornerX = px < cxL || px > cxR;
            const bool inCornerY = py < cyT || py > cyB;
            if (!(inCornerX && inCornerY)) {
                blendPixel(x, y, c);
                continue;
            }

            const float cx = px < cxL ? cxL : cxR;
            const float cy = py < cyT ? cyT : cyB;
            const float dist = std::sqrt((px - cx) * (px - cx) + (py - cy) * (py - cy));

            // One-pixel antialiased edge.
            const float coverage = std::clamp(rad + 0.5f - dist, 0.0f, 1.0f);
            if (coverage > 0.0f)
                blendPixel(x, y, c, static_cast<uint8_t>(coverage * 255.0f + 0.5f));
        }
    }
}

} // namespace vpp
