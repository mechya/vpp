#include "vpp/font.h"

#define STB_TRUETYPE_IMPLEMENTATION
#define STBTT_STATIC
#include <stb_truetype.h>

#include <cstdio>
#include <unordered_map>
#include <vector>

namespace vpp {

namespace {

struct Glyph {
    int width = 0;
    int height = 0;
    int offsetX = 0;   // left bearing from pen position
    int offsetY = 0;   // top offset from baseline (negative = above)
    int advance = 0;
    std::vector<unsigned char> bitmap; // width * height coverage values
};

uint64_t glyphKey(int codepoint, float pixelSize) {
    return (static_cast<uint64_t>(static_cast<uint32_t>(codepoint)) << 32) |
           static_cast<uint32_t>(pixelSize * 64.0f);
}

} // namespace

struct Font::Impl {
    std::vector<unsigned char> data;
    stbtt_fontinfo info{};
    bool ok = false;
    mutable std::unordered_map<uint64_t, Glyph> cache;

    const Glyph& glyph(int codepoint, float pixelSize) const {
        const uint64_t key = glyphKey(codepoint, pixelSize);
        auto it = cache.find(key);
        if (it != cache.end()) return it->second;

        const float scale = stbtt_ScaleForPixelHeight(&info, pixelSize);
        Glyph g;

        int advance = 0;
        int lsb = 0;
        stbtt_GetCodepointHMetrics(&info, codepoint, &advance, &lsb);
        g.advance = static_cast<int>(advance * scale + 0.5f);

        int w = 0, h = 0, ox = 0, oy = 0;
        unsigned char* bmp = stbtt_GetCodepointBitmap(&info, 0, scale, codepoint, &w, &h, &ox, &oy);
        if (bmp) {
            g.width = w;
            g.height = h;
            g.offsetX = ox;
            g.offsetY = oy;
            g.bitmap.assign(bmp, bmp + static_cast<size_t>(w) * h);
            stbtt_FreeBitmap(bmp, nullptr);
        }

        return cache.emplace(key, std::move(g)).first->second;
    }
};

Font::Font() : impl_(new Impl) {}
Font::~Font() = default;

bool Font::load(const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) return false;

    std::fseek(f, 0, SEEK_END);
    const long size = std::ftell(f);
    std::fseek(f, 0, SEEK_SET);
    if (size <= 0) {
        std::fclose(f);
        return false;
    }

    impl_->data.resize(static_cast<size_t>(size));
    const size_t read = std::fread(impl_->data.data(), 1, impl_->data.size(), f);
    std::fclose(f);
    if (read != impl_->data.size()) return false;

    const int offset = stbtt_GetFontOffsetForIndex(impl_->data.data(), 0);
    impl_->ok = offset >= 0 && stbtt_InitFont(&impl_->info, impl_->data.data(), offset) != 0;
    impl_->cache.clear();
    return impl_->ok;
}

bool Font::loaded() const {
    return impl_->ok;
}

int Font::measure(const std::string& text, float pixelSize) const {
    if (!impl_->ok) return 0;
    int width = 0;
    for (unsigned char ch : text)
        width += impl_->glyph(ch, pixelSize).advance;
    return width;
}

float Font::ascent(float pixelSize) const {
    if (!impl_->ok) return 0.0f;
    int a = 0, d = 0, gap = 0;
    stbtt_GetFontVMetrics(&impl_->info, &a, &d, &gap);
    return a * stbtt_ScaleForPixelHeight(&impl_->info, pixelSize);
}

float Font::lineHeight(float pixelSize) const {
    if (!impl_->ok) return pixelSize;
    int a = 0, d = 0, gap = 0;
    stbtt_GetFontVMetrics(&impl_->info, &a, &d, &gap);
    return (a - d + gap) * stbtt_ScaleForPixelHeight(&impl_->info, pixelSize);
}

void Font::draw(Canvas& canvas, int x, int baselineY, const std::string& text, float pixelSize,
                Color color) const {
    if (!impl_->ok) return;

    int pen = x;
    for (unsigned char ch : text) {
        const Glyph& g = impl_->glyph(ch, pixelSize);
        for (int row = 0; row < g.height; ++row) {
            const unsigned char* src = &g.bitmap[static_cast<size_t>(row) * g.width];
            for (int col = 0; col < g.width; ++col) {
                if (src[col] == 0) continue;
                canvas.blendPixel(pen + g.offsetX + col, baselineY + g.offsetY + row, color, src[col]);
            }
        }
        pen += g.advance;
    }
}

} // namespace vpp
