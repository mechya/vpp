#pragma once

#include "vpp/dom.h"

#include <string>
#include <vector>

namespace vpp {

struct Declaration {
    std::string property; // lower-case
    std::string value;    // trimmed, as written
    bool important = false;
};

// One compound selector such as "button.primary#go" or "*".
struct CompoundSelector {
    std::string tag; // empty matches any element
    std::string id;
    std::vector<std::string> classes;
};

// A selector with combinators, e.g. ".card > p b". compounds[0] is the
// leftmost; combinators[i] joins compounds[i] and compounds[i + 1]:
// ' ' for descendant, '>' for child.
struct Selector {
    std::vector<CompoundSelector> compounds;
    std::vector<char> combinators;
    int specificity = 0; // ids * 10000 + classes * 100 + tags
};

struct Rule {
    std::vector<Selector> selectors;
    std::vector<Declaration> declarations;
};

struct StyleSheet {
    std::vector<Rule> rules;

    void append(StyleSheet other);
    bool empty() const { return rules.empty(); }
};

// Parses a stylesheet. Unsupported constructs (at-rules, pseudo-classes,
// attribute selectors) are skipped rather than failing the whole sheet.
StyleSheet parseStyleSheet(const std::string& css);

// Parses the body of a style="" attribute.
std::vector<Declaration> parseDeclarations(const std::string& text);

bool selectorMatches(const Selector& selector, const Node& element);

} // namespace vpp
