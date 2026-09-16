#pragma once

#include <cstdint>
#include <vector>

namespace vpp {

struct Rect {
    int x = 0;
    int y = 0;
    int w = 0;
    int h = 0;

    bool contains(int px, int py) const {
        return px >= x && py >= y && px < x + w && py < y + h;
    }
};

struct Color {
    uint8_t r = 0;
    uint8_t g = 0;
    uint8_t b = 0;
    uint8_t a = 255;

    static constexpr Color rgb(uint8_t r, uint8_t g, uint8_t b) { return {r, g, b, 255}; }
    static constexpr Color rgba(uint8_t r, uint8_t g, uint8_t b, uint8_t a) { return {r, g, b, a}; }
};

// A CPU pixel buffer in ARGB8888 (0xAARRGGBB), the format SDL presents directly.
class Canvas {
public:
    Canvas(int width, int height);

    void resize(int width, int height);

    int width() const { return width_; }
    int height() const { return height_; }
    int pitch() const { return width_ * static_cast<int>(sizeof(uint32_t)); }
    const uint32_t* pixels() const { return pixels_.data(); }

    void clear(Color c);
    void fillRect(Rect r, Color c);
    void fillRoundRect(Rect r, int radius, Color c);

    // Blends c onto (x, y). coverage scales the alpha, 0..255.
    void blendPixel(int x, int y, Color c, uint8_t coverage = 255);

    // Makes the pixels outside a rounded rectangle covering the whole canvas
    // transparent, with antialiased corners. For rounded window corners.
    void maskRoundedCorners(int radius);

private:
    int width_;
    int height_;
    std::vector<uint32_t> pixels_;
};

} // namespace vpp
