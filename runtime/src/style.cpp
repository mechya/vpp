// Style resolution: user-agent defaults, the cascade, inheritance, and
// turning declared values into numbers layout can use.

#include "vpp/style.h"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <map>
#include <string>
#include <vector>

namespace vpp {

float Length::resolve(float reference, float fallback) const {
    switch (unit) {
    case Unit::Px: return value;
    case Unit::Percent: return reference * value / 100.0f;
    case Unit::Auto: break;
    }
    return fallback;
}

namespace {

const Color kAccent = Color::rgb(37, 99, 235);
const Color kWhite = Color::rgb(255, 255, 255);
constexpr float kRootFontSize = 16.0f;

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

std::vector<std::string> tokens(const std::string& value) {
    std::vector<std::string> out;
    std::string cur;
    int depth = 0;
    for (char c : value) {
        if (c == '(') ++depth;
        if (c == ')') --depth;
        if (std::isspace(static_cast<unsigned char>(c)) && depth == 0) {
            if (!cur.empty()) out.push_back(cur);
            cur.clear();
        } else {
            cur.push_back(c);
        }
    }
    if (!cur.empty()) out.push_back(cur);
    return out;
}

bool parseNumber(const std::string& s, float& out, size_t* end = nullptr) {
    size_t i = 0;
    if (i < s.size() && (s[i] == '-' || s[i] == '+')) ++i;
    bool digits = false;
    while (i < s.size() && (std::isdigit(static_cast<unsigned char>(s[i])) || s[i] == '.')) {
        if (s[i] != '.') digits = true;
        ++i;
    }
    if (!digits) return false;
    out = std::strtof(s.substr(0, i).c_str(), nullptr);
    if (end) *end = i;
    return true;
}

// Parses a length. em is relative to fontSize.
bool parseLength(const std::string& raw, float fontSize, Length& out) {
    const std::string s = lower(raw);
    if (s == "auto") {
        out = Length{};
        return true;
    }
    float num = 0;
    size_t end = 0;
    if (!parseNumber(s, num, &end)) return false;
    const std::string unit = s.substr(end);
    if (unit == "px" || unit.empty()) out = Length::px(num);
    else if (unit == "em") out = Length::px(num * fontSize);
    else if (unit == "rem") out = Length::px(num * kRootFontSize);
    else if (unit == "%") out = Length::percent(num);
    else return false;
    return true;
}

bool parseHex(const std::string& hex, Color& out) {
    auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        return -1;
    };
    std::vector<int> v;
    for (char c : hex) {
        const int n = nibble(c);
        if (n < 0) return false;
        v.push_back(n);
    }
    if (v.size() == 3 || v.size() == 4) {
        out.r = static_cast<uint8_t>(v[0] * 17);
        out.g = static_cast<uint8_t>(v[1] * 17);
        out.b = static_cast<uint8_t>(v[2] * 17);
        out.a = v.size() == 4 ? static_cast<uint8_t>(v[3] * 17) : 255;
        return true;
    }
    if (v.size() == 6 || v.size() == 8) {
        out.r = static_cast<uint8_t>(v[0] * 16 + v[1]);
        out.g = static_cast<uint8_t>(v[2] * 16 + v[3]);
        out.b = static_cast<uint8_t>(v[4] * 16 + v[5]);
        out.a = v.size() == 8 ? static_cast<uint8_t>(v[6] * 16 + v[7]) : 255;
        return true;
    }
    return false;
}

bool parseColor(const std::string& raw, Color& out) {
    static const std::map<std::string, Color> named = {
        {"black", Color::rgb(0, 0, 0)},         {"white", Color::rgb(255, 255, 255)},
        {"red", Color::rgb(255, 0, 0)},         {"green", Color::rgb(0, 128, 0)},
        {"blue", Color::rgb(0, 0, 255)},        {"yellow", Color::rgb(255, 255, 0)},
        {"orange", Color::rgb(255, 165, 0)},    {"purple", Color::rgb(128, 0, 128)},
        {"pink", Color::rgb(255, 192, 203)},    {"gray", Color::rgb(128, 128, 128)},
        {"grey", Color::rgb(128, 128, 128)},    {"silver", Color::rgb(192, 192, 192)},
        {"navy", Color::rgb(0, 0, 128)},        {"teal", Color::rgb(0, 128, 128)},
        {"maroon", Color::rgb(128, 0, 0)},      {"olive", Color::rgb(128, 128, 0)},
        {"lime", Color::rgb(0, 255, 0)},        {"aqua", Color::rgb(0, 255, 255)},
        {"cyan", Color::rgb(0, 255, 255)},      {"fuchsia", Color::rgb(255, 0, 255)},
        {"magenta", Color::rgb(255, 0, 255)},   {"brown", Color::rgb(165, 42, 42)},
        {"transparent", Color::rgba(0, 0, 0, 0)},
    };

    const std::string s = lower(raw);
    auto it = named.find(s);
    if (it != named.end()) {
        out = it->second;
        return true;
    }
    if (!s.empty() && s[0] == '#') return parseHex(s.substr(1), out);

    const bool rgb = s.rfind("rgb(", 0) == 0;
    const bool rgba = s.rfind("rgba(", 0) == 0;
    if ((rgb || rgba) && s.back() == ')') {
        const std::string inner = s.substr(rgb ? 4 : 5, s.size() - (rgb ? 5 : 6));
        std::vector<float> parts;
        std::string cur;
        for (char c : inner + ",") {
            if (c == ',' || c == '/' || std::isspace(static_cast<unsigned char>(c))) {
                float v = 0;
                if (!cur.empty() && parseNumber(cur, v)) parts.push_back(cur.back() == '%' ? v * 2.55f : v);
                cur.clear();
            } else {
                cur.push_back(c);
            }
        }
        if (parts.size() < 3) return false;
        auto clamp255 = [](float v) { return static_cast<uint8_t>(std::clamp(v, 0.0f, 255.0f) + 0.5f); };
        out.r = clamp255(parts[0]);
        out.g = clamp255(parts[1]);
        out.b = clamp255(parts[2]);
        out.a = parts.size() > 3 ? clamp255(parts[3] <= 1.0f ? parts[3] * 255.0f : parts[3]) : 255;
        return true;
    }
    return false;
}

// margin / padding shorthand: 1 to 4 values in CSS order.
struct EdgeValues {
    Length top, right, bottom, left;
};

bool parseEdges(const std::vector<std::string>& t, float fontSize, EdgeValues& out) {
    std::vector<Length> v;
    for (const std::string& s : t) {
        Length l;
        if (!parseLength(s, fontSize, l)) return false;
        v.push_back(l);
    }
    switch (v.size()) {
    case 1: out = {v[0], v[0], v[0], v[0]}; return true;
    case 2: out = {v[0], v[1], v[0], v[1]}; return true;
    case 3: out = {v[0], v[1], v[2], v[1]}; return true;
    case 4: out = {v[0], v[1], v[2], v[3]}; return true;
    default: return false;
    }
}

void applyUserAgentStyle(const Node& element, const ComputedStyle& parent, ComputedStyle& s) {
    s.color = parent.color;
    s.fontSize = parent.fontSize;
    s.bold = parent.bold;
    s.textAlign = parent.textAlign;

    const std::string& tag = element.tag();

    if (tag == "html") {
        s.display = Display::Block;
    } else if (tag == "body") {
        s.display = Display::Block;
        s.margin = Edges::all(16);
    } else if (tag == "head" || tag == "title" || tag == "script" || tag == "style" || tag == "meta" ||
               tag == "link") {
        s.display = Display::None;
    } else if (tag == "h1") {
        s.display = Display::Block;
        s.fontSize = 32;
        s.bold = true;
        s.margin = Edges::vertical(21);
    } else if (tag == "h2") {
        s.display = Display::Block;
        s.fontSize = 24;
        s.bold = true;
        s.margin = Edges::vertical(20);
    } else if (tag == "h3") {
        s.display = Display::Block;
        s.fontSize = 19;
        s.bold = true;
        s.margin = Edges::vertical(19);
    } else if (tag == "p") {
        s.display = Display::Block;
        s.margin = Edges::vertical(16);
    } else if (tag == "div" || tag == "section" || tag == "main" || tag == "header" || tag == "footer" ||
               tag == "nav" || tag == "article" || tag == "ul" || tag == "ol" || tag == "li" ||
               tag == "form" || tag == "aside") {
        s.display = Display::Block;
    } else if (tag == "button") {
        s.display = Display::InlineBlock;
        s.color = kWhite;
        s.bold = false;
        s.hasBackground = true;
        s.background = kAccent;
        s.padding = Edges::symmetric(12, 24);
        s.borderRadius = 8;
        s.textAlign = TextAlign::Center;
    } else if (tag == "b" || tag == "strong") {
        s.display = Display::Inline;
        s.bold = true;
    } else {
        s.display = Display::Inline;
    }
}

void applyDeclaration(const Declaration& d, const ComputedStyle& parent, ComputedStyle& s) {
    const std::string& p = d.property;
    const std::string v = lower(d.value);
    const std::vector<std::string> t = tokens(d.value);
    if (t.empty()) return;

    Length len;
    Color color;

    if (p == "display") {
        if (v == "none") s.display = Display::None;
        else if (v == "block") s.display = Display::Block;
        else if (v == "inline") s.display = Display::Inline;
        else if (v == "inline-block") s.display = Display::InlineBlock;
        else if (v == "flex") s.display = Display::Flex;
    } else if (p == "color") {
        if (parseColor(v, color)) s.color = color;
    } else if (p == "background-color" || p == "background") {
        for (const std::string& tok : t) {
            if (parseColor(tok, color)) {
                s.hasBackground = color.a > 0;
                s.background = color;
                break;
            }
        }
    } else if (p == "font-size") {
        if (parseLength(v, parent.fontSize, len)) s.fontSize = len.resolve(parent.fontSize, parent.fontSize);
    } else if (p == "font-weight") {
        float n = 0;
        if (v == "bold" || v == "bolder") s.bold = true;
        else if (v == "normal" || v == "lighter") s.bold = false;
        else if (parseNumber(v, n)) s.bold = n >= 600;
    } else if (p == "text-align") {
        if (v == "left" || v == "start") s.textAlign = TextAlign::Left;
        else if (v == "center") s.textAlign = TextAlign::Center;
        else if (v == "right" || v == "end") s.textAlign = TextAlign::Right;
    } else if (p == "margin") {
        EdgeValues e;
        if (parseEdges(t, s.fontSize, e)) {
            s.margin = {e.top.resolve(0), e.right.resolve(0), e.bottom.resolve(0), e.left.resolve(0)};
            s.marginLeftAuto = e.left.isAuto();
            s.marginRightAuto = e.right.isAuto();
        }
    } else if (p == "margin-top" || p == "margin-right" || p == "margin-bottom" || p == "margin-left") {
        if (parseLength(v, s.fontSize, len)) {
            const float px = len.resolve(0);
            if (p == "margin-top") s.margin.top = px;
            else if (p == "margin-bottom") s.margin.bottom = px;
            else if (p == "margin-left") { s.margin.left = px; s.marginLeftAuto = len.isAuto(); }
            else { s.margin.right = px; s.marginRightAuto = len.isAuto(); }
        }
    } else if (p == "padding") {
        EdgeValues e;
        if (parseEdges(t, s.fontSize, e))
            s.padding = {e.top.resolve(0), e.right.resolve(0), e.bottom.resolve(0), e.left.resolve(0)};
    } else if (p == "padding-top" || p == "padding-right" || p == "padding-bottom" || p == "padding-left") {
        if (parseLength(v, s.fontSize, len)) {
            const float px = len.resolve(0);
            if (p == "padding-top") s.padding.top = px;
            else if (p == "padding-right") s.padding.right = px;
            else if (p == "padding-bottom") s.padding.bottom = px;
            else s.padding.left = px;
        }
    } else if (p == "border") {
        if (v == "none" || v == "0") {
            s.borderWidth = 0;
            return;
        }
        for (const std::string& tok : t) {
            if (parseLength(tok, s.fontSize, len)) s.borderWidth = len.resolve(0);
            else if (parseColor(tok, color)) s.borderColor = color;
            // "solid" and other border styles are accepted and ignored.
        }
        if (s.borderWidth == 0 && s.borderColor.a > 0) s.borderWidth = 1; // "border: solid red"
    } else if (p == "border-width") {
        if (parseLength(v, s.fontSize, len)) s.borderWidth = len.resolve(0);
    } else if (p == "border-color") {
        if (parseColor(v, color)) s.borderColor = color;
    } else if (p == "border-radius") {
        if (parseLength(t[0], s.fontSize, len)) s.borderRadius = len.resolve(0);
    } else if (p == "width") {
        if (parseLength(v, s.fontSize, len)) s.width = len;
    } else if (p == "height") {
        if (parseLength(v, s.fontSize, len)) s.height = len;
    } else if (p == "max-width") {
        if (v == "none") s.maxWidth = Length{};
        else if (parseLength(v, s.fontSize, len)) s.maxWidth = len;
    } else if (p == "flex-direction") {
        if (v == "row") s.flexDirection = FlexDirection::Row;
        else if (v == "column") s.flexDirection = FlexDirection::Column;
    } else if (p == "justify-content") {
        if (v == "flex-start" || v == "start" || v == "left") s.justifyContent = JustifyContent::Start;
        else if (v == "center") s.justifyContent = JustifyContent::Center;
        else if (v == "flex-end" || v == "end" || v == "right") s.justifyContent = JustifyContent::End;
        else if (v == "space-between") s.justifyContent = JustifyContent::SpaceBetween;
        else if (v == "space-around" || v == "space-evenly") s.justifyContent = JustifyContent::SpaceAround;
    } else if (p == "align-items") {
        if (v == "stretch") s.alignItems = AlignItems::Stretch;
        else if (v == "flex-start" || v == "start") s.alignItems = AlignItems::Start;
        else if (v == "center") s.alignItems = AlignItems::Center;
        else if (v == "flex-end" || v == "end") s.alignItems = AlignItems::End;
    } else if (p == "gap") {
        if (parseLength(t[0], s.fontSize, len)) s.gap = len.resolve(0);
    } else if (p == "flex-grow") {
        float n = 0;
        if (parseNumber(v, n)) s.flexGrow = std::max(n, 0.0f);
    } else if (p == "flex") {
        float n = 0;
        if (v == "none" || v == "initial") s.flexGrow = 0;
        else if (v == "auto") s.flexGrow = 1;
        else if (parseNumber(t[0], n)) s.flexGrow = std::max(n, 0.0f);
    }
    // Unknown properties are ignored, as in a browser.
}

} // namespace

ComputedStyle initialStyle() {
    return ComputedStyle{};
}

ComputedStyle computeStyle(const Node& element, const ComputedStyle& parent, const StyleSheet* sheet) {
    ComputedStyle s;
    applyUserAgentStyle(element, parent, s);

    // Gather the declarations that apply, in cascade order.
    struct Match {
        bool important;
        int specificity;
        int order;
        const Declaration* decl;
    };
    std::vector<Match> matches;
    int order = 0;

    if (sheet) {
        for (const Rule& rule : sheet->rules) {
            int best = -1;
            for (const Selector& sel : rule.selectors)
                if (selectorMatches(sel, element)) best = std::max(best, sel.specificity);
            if (best < 0) continue;
            for (const Declaration& d : rule.declarations)
                matches.push_back({d.important, best, order++, &d});
        }
    }

    std::vector<Declaration> inlineDecls;
    if (const std::string* attr = element.attribute("style")) {
        inlineDecls = parseDeclarations(*attr);
        for (const Declaration& d : inlineDecls)
            matches.push_back({d.important, 1000000, order++, &d});
    }

    std::stable_sort(matches.begin(), matches.end(), [](const Match& a, const Match& b) {
        if (a.important != b.important) return !a.important;
        if (a.specificity != b.specificity) return a.specificity < b.specificity;
        return a.order < b.order;
    });

    // font-size first, so em lengths on the same element resolve against it.
    for (const Match& m : matches)
        if (m.decl->property == "font-size") applyDeclaration(*m.decl, parent, s);
    for (const Match& m : matches)
        if (m.decl->property != "font-size") applyDeclaration(*m.decl, parent, s);

    return s;
}

} // namespace vpp
