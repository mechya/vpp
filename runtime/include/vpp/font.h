#pragma once

#include "vpp/canvas.h"

#include <memory>
#include <string>

namespace vpp {

// A TrueType font loaded from disk and rasterised on demand.
class Font {
public:
    Font();
    ~Font();

    Font(const Font&) = delete;
    Font& operator=(const Font&) = delete;

    bool load(const std::string& path);
    bool loaded() const;

    // Width of text in pixels at the given pixel size.
    int measure(const std::string& text, float pixelSize) const;

    // Distance from the top of a line to its baseline, and full line height.
    float ascent(float pixelSize) const;
    float lineHeight(float pixelSize) const;

    // Draws text with its baseline at y.
    void draw(Canvas& canvas, int x, int baselineY, const std::string& text, float pixelSize,
              Color color) const;

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

// A regular and bold face. Bold falls back to regular when not loaded.
struct FontSet {
    Font regular;
    Font bold;

    const Font& pick(bool wantBold) const { return (wantBold && bold.loaded()) ? bold : regular; }
    bool loaded() const { return regular.loaded(); }
};

} // namespace vpp
