// Layout.
//
// Block containers stack their block-level children vertically with
// collapsing vertical margins, and break inline content (text, inline
// elements, inline-level boxes) into lines. Flex containers place their
// children along a row or column. Inline-level boxes (inline-block, or any
// element placed in inline content) are laid out as shrink-to-fit blocks.

#include "vpp/layout.h"

#include <algorithm>
#include <cctype>

namespace vpp {

namespace {

struct Insets {
    float top = 0;
    float right = 0;
    float bottom = 0;
    float left = 0;
};

Insets insetsOf(const ComputedStyle& s) {
    const float b = s.borderWidth;
    return {s.padding.top + b, s.padding.right + b, s.padding.bottom + b, s.padding.left + b};
}

struct InlineItem {
    enum class Kind { Word, Box };

    Kind kind = Kind::Word;
    std::string text;
    ComputedStyle style;
    float width = 0;
    float ascent = 0;
    float height = 0;
    bool spaceBefore = false;
    LayoutBox* box = nullptr; // owned by the container's children
    float lineX = 0;          // assigned during line breaking
};

template <typename Fn>
void forEachWord(const std::string& text, Fn&& fn) {
    std::string word;
    bool space = false;
    for (unsigned char c : text) {
        if (std::isspace(c)) {
            if (!word.empty()) {
                fn(word, space);
                word.clear();
                space = false;
            }
            space = true;
        } else {
            word.push_back(static_cast<char>(c));
        }
    }
    if (!word.empty()) fn(word, space);
    else if (space) fn(std::string(), true); // trailing whitespace still separates words
}

std::unique_ptr<LayoutBox> makeAnonymous(const ComputedStyle& parent) {
    auto box = std::make_unique<LayoutBox>();
    box->style = parent;
    box->style.display = Display::Block;
    box->style.hasBackground = false;
    box->style.margin = Edges{};
    box->style.padding = Edges{};
    box->style.borderWidth = 0;
    box->style.borderRadius = 0;
    box->style.width = Length{};
    box->style.height = Length{};
    box->style.maxWidth = Length{};
    return box;
}

bool isBlockLevel(Display d) {
    return d == Display::Block || d == Display::Flex;
}

class Layouter {
public:
    explicit Layouter(const LayoutContext& ctx) : ctx_(ctx) {}

    // Lays out a block-level box inside a container: resolves its width and
    // horizontal margins, then its content. Vertical margins are the caller's.
    void layoutBlockLevel(LayoutBox& box, float containerX, float y, float containerWidth);

    // Lays out a box with a known border-box width at (x, y). Sets frame.h.
    void layoutWithWidth(LayoutBox& box, float x, float y, float borderBoxWidth);

    // Border-box width of an element's content laid out on one line.
    float maxContentWidth(const Node& node, const ComputedStyle& style);

private:
    ComputedStyle styleOf(const Node& n, const ComputedStyle& parent) const {
        return computeStyle(n, parent, ctx_.sheet);
    }
    const Font& fontFor(const ComputedStyle& s) const { return ctx_.fonts.pick(s.bold); }
    float devicePx(const ComputedStyle& s) const { return s.fontSize * ctx_.scale; }
    float textWidth(const ComputedStyle& s, const std::string& text) const {
        return fontFor(s).measure(text, devicePx(s)) / ctx_.scale;
    }
    float ascent(const ComputedStyle& s) const { return fontFor(s).ascent(devicePx(s)) / ctx_.scale; }
    float lineHeight(const ComputedStyle& s) const {
        return fontFor(s).lineHeight(devicePx(s)) / ctx_.scale;
    }

    void layoutContent(LayoutBox& box);
    void layoutBlockContent(LayoutBox& box);
    void layoutFlexContent(LayoutBox& box);
    void layoutInlineLevel(LayoutBox& box, float availableWidth);

    void collectInline(const Node& node, const ComputedStyle& style, LayoutBox& container,
                       std::vector<InlineItem>& items, bool& pendingSpace, float availableWidth);
    float layoutLines(LayoutBox& container, std::vector<InlineItem>& items, float x, float y, float width,
                      TextAlign align);

    void measureInlineRun(const Node& parent, const ComputedStyle& style, float& run, float& best,
                          bool& pendingSpace);

    static void translate(LayoutBox& box, float dx, float dy);
    static bool firstBaseline(const LayoutBox& box, float& out);

    const LayoutContext& ctx_;
};

// --- sizing ------------------------------------------------------------------

void Layouter::measureInlineRun(const Node& parent, const ComputedStyle& style, float& run, float& best,
                                bool& pendingSpace) {
    for (const auto& child : parent.children()) {
        if (child->isText()) {
            forEachWord(child->text(), [&](const std::string& word, bool spaceBefore) {
                if (word.empty()) {
                    pendingSpace = true;
                    return;
                }
                if (run > 0 && (spaceBefore || pendingSpace)) run += textWidth(style, " ");
                run += textWidth(style, word);
                pendingSpace = false;
            });
            continue;
        }
        if (!child->isElement()) continue;

        const ComputedStyle cs = styleOf(*child, style);
        switch (cs.display) {
        case Display::None:
            break;
        case Display::Inline:
            measureInlineRun(*child, cs, run, best, pendingSpace);
            break;
        case Display::InlineBlock:
            if (run > 0 && pendingSpace) run += textWidth(style, " ");
            run += maxContentWidth(*child, cs) + cs.margin.left + cs.margin.right;
            pendingSpace = false;
            break;
        case Display::Block:
        case Display::Flex:
            best = std::max(best, run);
            run = 0;
            pendingSpace = false;
            best = std::max(best, maxContentWidth(*child, cs) + cs.margin.left + cs.margin.right);
            break;
        }
    }
}

float Layouter::maxContentWidth(const Node& node, const ComputedStyle& style) {
    const Insets in = insetsOf(style);
    if (style.width.unit == Length::Unit::Px) return style.width.value + in.left + in.right;

    float content = 0;
    if (style.display == Display::Flex) {
        const bool row = style.flexDirection == FlexDirection::Row;
        int count = 0;
        for (const auto& child : node.children()) {
            if (!child->isElement()) continue;
            const ComputedStyle cs = styleOf(*child, style);
            if (cs.display == Display::None) continue;
            const float w = maxContentWidth(*child, cs) + cs.margin.left + cs.margin.right;
            if (row) content += w;
            else content = std::max(content, w);
            ++count;
        }
        if (row && count > 1) content += style.gap * static_cast<float>(count - 1);
    } else {
        float run = 0;
        float best = 0;
        bool pendingSpace = false;
        measureInlineRun(node, style, run, best, pendingSpace);
        content = std::max(best, run);
    }

    if (style.maxWidth.unit == Length::Unit::Px) content = std::min(content, style.maxWidth.value);
    return content + in.left + in.right;
}

// --- block-level -------------------------------------------------------------

void Layouter::layoutBlockLevel(LayoutBox& box, float containerX, float y, float containerWidth) {
    const ComputedStyle& s = box.style;
    const Insets in = insetsOf(s);

    float ml = s.marginLeftAuto ? 0 : s.margin.left;
    float mr = s.marginRightAuto ? 0 : s.margin.right;

    float w;
    if (s.width.isAuto()) w = containerWidth - ml - mr;
    else w = s.width.resolve(containerWidth) + in.left + in.right;
    if (!s.maxWidth.isAuto()) w = std::min(w, s.maxWidth.resolve(containerWidth) + in.left + in.right);
    w = std::max(w, 0.0f);

    const float remaining = containerWidth - w - ml - mr;
    if (remaining > 0) {
        if (s.marginLeftAuto && s.marginRightAuto) ml += remaining / 2;
        else if (s.marginLeftAuto) ml += remaining;
    }

    layoutWithWidth(box, containerX + ml, y, w);
}

void Layouter::layoutWithWidth(LayoutBox& box, float x, float y, float borderBoxWidth) {
    box.frame = {x, y, borderBoxWidth, 0};
    box.children.clear();
    box.lines.clear();
    layoutContent(box);

    const ComputedStyle& s = box.style;
    if (s.height.unit == Length::Unit::Px) {
        const Insets in = insetsOf(s);
        box.frame.h = s.height.value + in.top + in.bottom;
    }
}

void Layouter::layoutContent(LayoutBox& box) {
    if (box.style.display == Display::Flex) layoutFlexContent(box);
    else layoutBlockContent(box);
}

void Layouter::layoutInlineLevel(LayoutBox& box, float availableWidth) {
    const ComputedStyle& s = box.style;
    const Insets in = insetsOf(s);

    float w;
    if (s.width.isAuto()) w = std::min(maxContentWidth(*box.node, s), availableWidth);
    else w = s.width.resolve(availableWidth) + in.left + in.right;
    if (!s.maxWidth.isAuto()) w = std::min(w, s.maxWidth.resolve(availableWidth) + in.left + in.right);

    layoutWithWidth(box, 0, 0, std::max(w, 0.0f));

    float baseline = 0;
    box.baseline = firstBaseline(box, baseline) ? baseline : box.frame.h;
}

// --- block content: block children and inline runs ----------------------------

void Layouter::layoutBlockContent(LayoutBox& box) {
    const ComputedStyle& s = box.style;
    const Insets in = insetsOf(s);

    const float contentX = box.frame.x + in.left;
    const float contentWidth = std::max(box.frame.w - in.left - in.right, 0.0f);
    float cursorY = box.frame.y + in.top;
    float pendingMargin = 0;
    bool hadBlockChild = false;

    std::vector<InlineItem> items;
    bool pendingSpace = false;
    LayoutBox* anonymous = nullptr;

    auto flushInline = [&] {
        if (anonymous) {
            if (items.empty()) {
                box.children.pop_back(); // whitespace-only run
            } else {
                cursorY += pendingMargin;
                pendingMargin = 0;
                const float h = layoutLines(*anonymous, items, contentX, cursorY, contentWidth, s.textAlign);
                anonymous->frame = {contentX, cursorY, contentWidth, h};
                cursorY += h;
            }
        }
        items.clear();
        anonymous = nullptr;
        pendingSpace = false;
    };

    auto ensureAnonymous = [&]() -> LayoutBox& {
        if (!anonymous) {
            box.children.push_back(makeAnonymous(s));
            anonymous = box.children.back().get();
        }
        return *anonymous;
    };

    for (const auto& child : box.node->children()) {
        if (child->isText()) {
            collectInline(*child, s, ensureAnonymous(), items, pendingSpace, contentWidth);
            continue;
        }
        if (!child->isElement()) continue;

        const ComputedStyle cs = styleOf(*child, s);
        if (cs.display == Display::None) continue;

        if (isBlockLevel(cs.display)) {
            flushInline();
            auto childBox = std::make_unique<LayoutBox>();
            childBox->node = child.get();
            childBox->style = cs;

            cursorY += std::max(pendingMargin, cs.margin.top);
            layoutBlockLevel(*childBox, contentX, cursorY, contentWidth);
            cursorY += childBox->frame.h;
            pendingMargin = cs.margin.bottom;
            hadBlockChild = true;

            box.children.push_back(std::move(childBox));
        } else {
            collectInline(*child, s, ensureAnonymous(), items, pendingSpace, contentWidth);
        }
    }
    flushInline();

    if (hadBlockChild) cursorY += pendingMargin;
    box.frame.h = cursorY - box.frame.y + in.bottom;
}

void Layouter::collectInline(const Node& node, const ComputedStyle& style, LayoutBox& container,
                             std::vector<InlineItem>& items, bool& pendingSpace, float availableWidth) {
    if (node.isText()) {
        forEachWord(node.text(), [&](const std::string& word, bool spaceBefore) {
            if (word.empty()) {
                pendingSpace = true;
                return;
            }
            InlineItem item;
            item.kind = InlineItem::Kind::Word;
            item.text = word;
            item.style = style;
            item.width = textWidth(style, word);
            item.ascent = ascent(style);
            item.height = lineHeight(style);
            item.spaceBefore = spaceBefore || pendingSpace;
            items.push_back(std::move(item));
            pendingSpace = false;
        });
        return;
    }
    if (!node.isElement()) return;

    const ComputedStyle cs = styleOf(node, style);
    if (cs.display == Display::None) return;

    if (cs.display == Display::Inline) {
        for (const auto& child : node.children())
            collectInline(*child, cs, container, items, pendingSpace, availableWidth);
        return;
    }

    // inline-block, or a block-level element inside inline content.
    auto box = std::make_unique<LayoutBox>();
    box->node = &node;
    box->style = cs;
    layoutInlineLevel(*box, availableWidth);

    InlineItem item;
    item.kind = InlineItem::Kind::Box;
    item.style = cs;
    item.width = box->frame.w + cs.margin.left + cs.margin.right;
    item.ascent = box->baseline + cs.margin.top;
    item.height = box->frame.h + cs.margin.top + cs.margin.bottom;
    item.spaceBefore = pendingSpace;
    item.box = box.get();
    items.push_back(std::move(item));
    container.children.push_back(std::move(box));
    pendingSpace = false;
}

float Layouter::layoutLines(LayoutBox& container, std::vector<InlineItem>& items, float x, float y,
                            float width, TextAlign align) {
    container.lines.clear();
    float cursorY = y;
    size_t i = 0;

    while (i < items.size()) {
        Line line;
        std::vector<size_t> members;
        float cursorX = 0;
        float lineAscent = 0;
        float lineDescent = 0;

        while (i < items.size()) {
            InlineItem& item = items[i];
            const float space = (!members.empty() && item.spaceBefore) ? textWidth(item.style, " ") : 0;
            if (!members.empty() && cursorX + space + item.width > width + 0.01f) break;

            item.lineX = cursorX + space;
            cursorX += space + item.width;
            lineAscent = std::max(lineAscent, item.ascent);
            lineDescent = std::max(lineDescent, item.height - item.ascent);
            members.push_back(i);
            ++i;
        }

        float shift = 0;
        if (width > cursorX) {
            if (align == TextAlign::Center) shift = (width - cursorX) / 2;
            else if (align == TextAlign::Right) shift = width - cursorX;
        }

        const float lineH = lineAscent + lineDescent;
        line.rect = {x + shift, cursorY, cursorX, lineH};

        for (size_t idx : members) {
            InlineItem& item = items[idx];
            Fragment f;
            f.rect = {x + shift + item.lineX, cursorY + lineAscent - item.ascent, item.width, item.height};
            f.baseline = item.ascent;
            if (item.kind == InlineItem::Kind::Word) {
                f.text = item.text;
                f.color = item.style.color;
                f.fontSize = item.style.fontSize;
                f.bold = item.style.bold;
            } else {
                f.box = item.box;
                translate(*item.box, f.rect.x + item.style.margin.left, f.rect.y + item.style.margin.top);
            }
            line.fragments.push_back(std::move(f));
        }

        cursorY += lineH;
        container.lines.push_back(std::move(line));
    }

    return cursorY - y;
}

// --- flex ---------------------------------------------------------------------

void Layouter::layoutFlexContent(LayoutBox& box) {
    const ComputedStyle& s = box.style;
    const Insets in = insetsOf(s);
    const float contentX = box.frame.x + in.left;
    const float contentY = box.frame.y + in.top;
    const float contentW = std::max(box.frame.w - in.left - in.right, 0.0f);

    struct Item {
        LayoutBox* box;
        float main = 0; // border-box size along the main axis
    };
    std::vector<Item> items;

    for (const auto& child : box.node->children()) {
        if (!child->isElement()) continue; // text directly inside a flex container is not supported yet
        const ComputedStyle cs = styleOf(*child, s);
        if (cs.display == Display::None) continue;
        auto childBox = std::make_unique<LayoutBox>();
        childBox->node = child.get();
        childBox->style = cs;
        items.push_back({childBox.get(), 0});
        box.children.push_back(std::move(childBox));
    }

    const size_t n = items.size();
    if (n == 0) {
        box.frame.h = in.top + in.bottom;
        return;
    }
    const float gaps = s.gap * static_cast<float>(n - 1);
    float contentH = 0;

    if (s.flexDirection == FlexDirection::Row) {
        float total = gaps;
        float sumGrow = 0;
        for (Item& it : items) {
            const ComputedStyle& cs = it.box->style;
            const Insets ci = insetsOf(cs);
            if (cs.width.isAuto()) it.main = maxContentWidth(*it.box->node, cs);
            else it.main = cs.width.resolve(contentW) + ci.left + ci.right;
            if (!cs.maxWidth.isAuto()) it.main = std::min(it.main, cs.maxWidth.resolve(contentW) + ci.left + ci.right);
            total += it.main + cs.margin.left + cs.margin.right;
            sumGrow += cs.flexGrow;
        }

        float free = contentW - total;
        if (free > 0 && sumGrow > 0) {
            for (Item& it : items) it.main += free * it.box->style.flexGrow / sumGrow;
            free = 0;
        } else if (free < 0) {
            float sumMain = 0;
            for (Item& it : items) sumMain += it.main;
            if (sumMain > 0)
                for (Item& it : items) it.main = std::max(it.main + free * it.main / sumMain, 0.0f);
            free = 0;
        }

        float offset = 0;
        float spacing = s.gap;
        if (free > 0) {
            switch (s.justifyContent) {
            case JustifyContent::Start: break;
            case JustifyContent::Center: offset = free / 2; break;
            case JustifyContent::End: offset = free; break;
            case JustifyContent::SpaceBetween:
                if (n > 1) spacing += free / static_cast<float>(n - 1);
                break;
            case JustifyContent::SpaceAround: {
                const float per = free / static_cast<float>(n);
                offset = per / 2;
                spacing += per;
                break;
            }
            }
        }

        float x = contentX + offset;
        float lineH = 0;
        for (Item& it : items) {
            const ComputedStyle& cs = it.box->style;
            layoutWithWidth(*it.box, x + cs.margin.left, contentY + cs.margin.top, it.main);
            x += cs.margin.left + it.main + cs.margin.right + spacing;
            lineH = std::max(lineH, it.box->frame.h + cs.margin.top + cs.margin.bottom);
        }

        for (Item& it : items) {
            const ComputedStyle& cs = it.box->style;
            const float outer = it.box->frame.h + cs.margin.top + cs.margin.bottom;
            switch (s.alignItems) {
            case AlignItems::Stretch:
                if (cs.height.isAuto()) it.box->frame.h = lineH - cs.margin.top - cs.margin.bottom;
                break;
            case AlignItems::Start: break;
            case AlignItems::Center: translate(*it.box, 0, (lineH - outer) / 2); break;
            case AlignItems::End: translate(*it.box, 0, lineH - outer); break;
            }
        }
        contentH = lineH;
    } else {
        float cursorY = contentY;
        float sumGrow = 0;
        for (Item& it : items) {
            const ComputedStyle& cs = it.box->style;
            const Insets ci = insetsOf(cs);
            const float avail = std::max(contentW - cs.margin.left - cs.margin.right, 0.0f);

            float w;
            if (!cs.width.isAuto()) w = cs.width.resolve(contentW) + ci.left + ci.right;
            else if (s.alignItems == AlignItems::Stretch) w = avail;
            else w = std::min(maxContentWidth(*it.box->node, cs), avail);
            if (!cs.maxWidth.isAuto()) w = std::min(w, cs.maxWidth.resolve(contentW) + ci.left + ci.right);

            float xOff = 0;
            if (s.alignItems == AlignItems::Center) xOff = (avail - w) / 2;
            else if (s.alignItems == AlignItems::End) xOff = avail - w;

            layoutWithWidth(*it.box, contentX + cs.margin.left + xOff, cursorY + cs.margin.top, w);
            cursorY += cs.margin.top + it.box->frame.h + cs.margin.bottom + s.gap;
            sumGrow += cs.flexGrow;
        }
        contentH = cursorY - contentY - s.gap;

        // With an explicit container height, distribute the free space.
        if (s.height.unit == Length::Unit::Px) {
            const float innerH = s.height.value;
            float free = innerH - contentH;
            if (free > 0) {
                if (sumGrow > 0) {
                    float shift = 0;
                    for (Item& it : items) {
                        translate(*it.box, 0, shift);
                        const float extra = free * it.box->style.flexGrow / sumGrow;
                        it.box->frame.h += extra;
                        shift += extra;
                    }
                } else {
                    float offset = 0;
                    float extraSpacing = 0;
                    switch (s.justifyContent) {
                    case JustifyContent::Start: break;
                    case JustifyContent::Center: offset = free / 2; break;
                    case JustifyContent::End: offset = free; break;
                    case JustifyContent::SpaceBetween:
                        if (n > 1) extraSpacing = free / static_cast<float>(n - 1);
                        break;
                    case JustifyContent::SpaceAround:
                        extraSpacing = free / static_cast<float>(n);
                        offset = extraSpacing / 2;
                        break;
                    }
                    float shift = offset;
                    for (Item& it : items) {
                        translate(*it.box, 0, shift);
                        shift += extraSpacing;
                    }
                }
            }
        }
    }

    box.frame.h = contentH + in.top + in.bottom;
}

// --- helpers ------------------------------------------------------------------

void Layouter::translate(LayoutBox& box, float dx, float dy) {
    if (dx == 0 && dy == 0) return;
    box.frame.x += dx;
    box.frame.y += dy;
    for (Line& line : box.lines) {
        line.rect.x += dx;
        line.rect.y += dy;
        for (Fragment& f : line.fragments) {
            f.rect.x += dx;
            f.rect.y += dy;
        }
    }
    for (auto& child : box.children)
        translate(*child, dx, dy);
}

bool Layouter::firstBaseline(const LayoutBox& box, float& out) {
    if (!box.lines.empty() && !box.lines.front().fragments.empty()) {
        const Fragment& f = box.lines.front().fragments.front();
        out = f.rect.y + f.baseline - box.frame.y;
        return true;
    }
    for (const auto& child : box.children) {
        if (firstBaseline(*child, out)) {
            out += child->frame.y - box.frame.y;
            return true;
        }
    }
    return false;
}

} // namespace

std::unique_ptr<LayoutBox> layoutDocument(const Node& document, const LayoutContext& ctx,
                                          float viewportWidth) {
    const Node* html = nullptr;
    for (const auto& child : document.children()) {
        if (child->isElement()) {
            html = child.get();
            break;
        }
    }
    if (!html) return nullptr;

    auto root = std::make_unique<LayoutBox>();
    root->node = html;
    root->style = computeStyle(*html, initialStyle(), ctx.sheet);

    Layouter layouter(ctx);
    layouter.layoutBlockLevel(*root, 0, 0, viewportWidth);
    return root;
}

const LayoutBox* hitTest(const LayoutBox& root, float x, float y) {
    if (!root.frame.contains(x, y)) return nullptr;
    for (const auto& child : root.children)
        if (const LayoutBox* hit = hitTest(*child, x, y)) return hit;
    return root.node ? &root : nullptr;
}

} // namespace vpp
