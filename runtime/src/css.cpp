// CSS parsing and selector matching. A small hand-written parser for the
// subset VPP supports: rules with type, class, id, descendant and child
// selectors, plus plain declarations.

#include "vpp/css.h"

#include <algorithm>
#include <cctype>

namespace vpp {

namespace {

std::string trim(const std::string& s) {
    size_t a = 0;
    size_t b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

bool isNameChar(char c) {
    return std::isalnum(static_cast<unsigned char>(c)) || c == '-' || c == '_';
}

std::string stripComments(const std::string& css) {
    std::string out;
    out.reserve(css.size());
    size_t i = 0;
    while (i < css.size()) {
        if (css.compare(i, 2, "/*") == 0) {
            const size_t end = css.find("*/", i + 2);
            if (end == std::string::npos) break;
            out.push_back(' ');
            i = end + 2;
        } else {
            out.push_back(css[i++]);
        }
    }
    return out;
}

// Index of the '}' matching the '{' at open, or npos.
size_t matchingBrace(const std::string& s, size_t open) {
    int depth = 0;
    for (size_t i = open; i < s.size(); ++i) {
        if (s[i] == '{') ++depth;
        else if (s[i] == '}' && --depth == 0) return i;
    }
    return std::string::npos;
}

// Splits on sep at depth zero of parentheses and outside quotes.
std::vector<std::string> splitTopLevel(const std::string& s, char sep) {
    std::vector<std::string> parts;
    std::string cur;
    int depth = 0;
    char quote = 0;
    for (char c : s) {
        if (quote) {
            if (c == quote) quote = 0;
            cur.push_back(c);
        } else if (c == '"' || c == '\'') {
            quote = c;
            cur.push_back(c);
        } else if (c == '(') {
            ++depth;
            cur.push_back(c);
        } else if (c == ')') {
            --depth;
            cur.push_back(c);
        } else if (c == sep && depth == 0) {
            parts.push_back(cur);
            cur.clear();
        } else {
            cur.push_back(c);
        }
    }
    parts.push_back(cur);
    return parts;
}

bool parseSelector(const std::string& text, Selector& out) {
    Selector sel;
    CompoundSelector cur;
    bool haveCur = false;
    char pending = 0;
    int ids = 0, classes = 0, tags = 0;

    size_t i = 0;
    while (i < text.size()) {
        const char c = text[i];
        if (std::isspace(static_cast<unsigned char>(c))) {
            if (haveCur && pending == 0) pending = ' ';
            ++i;
            continue;
        }
        if (c == '>') {
            if (!haveCur) return false;
            pending = '>';
            ++i;
            continue;
        }

        if (haveCur && pending != 0) {
            sel.compounds.push_back(cur);
            sel.combinators.push_back(pending);
            cur = CompoundSelector{};
            pending = 0;
        }
        haveCur = true;

        if (c == '*') {
            ++i;
        } else if (c == '#' || c == '.') {
            size_t j = i + 1;
            while (j < text.size() && isNameChar(text[j])) ++j;
            if (j == i + 1) return false;
            const std::string name = text.substr(i + 1, j - i - 1);
            if (c == '#') {
                cur.id = name;
                ++ids;
            } else {
                cur.classes.push_back(name);
                ++classes;
            }
            i = j;
        } else if (isNameChar(c)) {
            size_t j = i;
            while (j < text.size() && isNameChar(text[j])) ++j;
            cur.tag = lower(text.substr(i, j - i));
            ++tags;
            i = j;
        } else {
            return false; // pseudo-classes, attributes, and anything else unsupported
        }
    }
    if (!haveCur) return false;
    sel.compounds.push_back(cur);
    sel.specificity = ids * 10000 + classes * 100 + tags;
    out = std::move(sel);
    return true;
}

std::vector<Selector> parseSelectorList(const std::string& text) {
    std::vector<Selector> out;
    for (const std::string& part : splitTopLevel(text, ',')) {
        Selector sel;
        if (parseSelector(trim(part), sel)) out.push_back(std::move(sel));
    }
    return out;
}

bool hasClass(const Node& element, const std::string& cls) {
    const std::string* attr = element.attribute("class");
    if (!attr) return false;
    size_t i = 0;
    while (i < attr->size()) {
        while (i < attr->size() && std::isspace(static_cast<unsigned char>((*attr)[i]))) ++i;
        size_t j = i;
        while (j < attr->size() && !std::isspace(static_cast<unsigned char>((*attr)[j]))) ++j;
        if (j > i && attr->compare(i, j - i, cls) == 0) return true;
        i = j;
    }
    return false;
}

bool compoundMatches(const CompoundSelector& c, const Node& element) {
    if (!element.isElement()) return false;
    if (!c.tag.empty() && c.tag != element.tag()) return false;
    if (!c.id.empty() && c.id != element.id()) return false;
    for (const std::string& cls : c.classes)
        if (!hasClass(element, cls)) return false;
    return true;
}

bool matchFrom(const Selector& sel, size_t index, const Node& element) {
    if (!compoundMatches(sel.compounds[index], element)) return false;
    if (index == 0) return true;

    const char combinator = sel.combinators[index - 1];
    if (combinator == '>') {
        const Node* parent = element.parent();
        return parent && parent->isElement() && matchFrom(sel, index - 1, *parent);
    }
    for (const Node* p = element.parent(); p; p = p->parent())
        if (p->isElement() && matchFrom(sel, index - 1, *p)) return true;
    return false;
}

} // namespace

void StyleSheet::append(StyleSheet other) {
    for (Rule& r : other.rules)
        rules.push_back(std::move(r));
}

std::vector<Declaration> parseDeclarations(const std::string& text) {
    std::vector<Declaration> out;
    for (const std::string& part : splitTopLevel(text, ';')) {
        const size_t colon = part.find(':');
        if (colon == std::string::npos) continue;
        Declaration d;
        d.property = lower(trim(part.substr(0, colon)));
        d.value = trim(part.substr(colon + 1));
        if (d.property.empty() || d.value.empty()) continue;

        const size_t bang = d.value.find('!');
        if (bang != std::string::npos && lower(trim(d.value.substr(bang + 1))) == "important") {
            d.important = true;
            d.value = trim(d.value.substr(0, bang));
        }
        out.push_back(std::move(d));
    }
    return out;
}

StyleSheet parseStyleSheet(const std::string& css) {
    StyleSheet sheet;
    const std::string text = stripComments(css);
    size_t i = 0;

    while (i < text.size()) {
        while (i < text.size() && std::isspace(static_cast<unsigned char>(text[i]))) ++i;
        if (i >= text.size()) break;

        if (text[i] == '@') {
            // Skip the at-rule: up to ';' or over its block.
            const size_t semi = text.find(';', i);
            const size_t brace = text.find('{', i);
            if (brace != std::string::npos && (semi == std::string::npos || brace < semi)) {
                const size_t close = matchingBrace(text, brace);
                if (close == std::string::npos) break;
                i = close + 1;
            } else if (semi != std::string::npos) {
                i = semi + 1;
            } else {
                break;
            }
            continue;
        }

        const size_t open = text.find('{', i);
        if (open == std::string::npos) break;
        const size_t close = matchingBrace(text, open);
        if (close == std::string::npos) break;

        Rule rule;
        rule.selectors = parseSelectorList(text.substr(i, open - i));
        rule.declarations = parseDeclarations(text.substr(open + 1, close - open - 1));
        if (!rule.selectors.empty() && !rule.declarations.empty()) sheet.rules.push_back(std::move(rule));
        i = close + 1;
    }
    return sheet;
}

bool selectorMatches(const Selector& selector, const Node& element) {
    if (selector.compounds.empty()) return false;
    return matchFrom(selector, selector.compounds.size() - 1, element);
}

} // namespace vpp
