#include "vpp/binary.h"

#include "vpp/bytes.h"

#include <cctype>

namespace vpp {

namespace {

constexpr uint16_t kFormatVersion = 1;
const char kDomMagic[4] = {'V', 'P', 'P', 'D'};
const char kStyleMagic[4] = {'V', 'P', 'P', 'S'};

enum NodeTag : uint8_t { kElement = 1, kText = 2 };

using Writer = ByteWriter;
using Reader = ByteReader;

bool isDroppedElement(const Node& n) {
    return n.isElement() && (n.tag() == "script" || n.tag() == "style" || n.tag() == "link");
}

// Collapses each run of whitespace to one space. Leading and trailing spaces
// are kept: they separate this text from neighbouring inline content.
std::string collapseWhitespace(const std::string& text) {
    std::string out;
    out.reserve(text.size());
    bool space = false;
    for (unsigned char c : text) {
        if (std::isspace(c)) {
            space = true;
        } else {
            if (space) out.push_back(' ');
            space = false;
            out.push_back(static_cast<char>(c));
        }
    }
    if (space) out.push_back(' ');
    return out;
}

uint32_t countKept(const Node& parent) {
    uint32_t n = 0;
    for (const auto& child : parent.children())
        if ((child->isElement() && !isDroppedElement(*child)) || child->isText()) ++n;
    return n;
}

void writeChildren(Writer& w, const Node& parent) {
    w.u32(countKept(parent));
    for (const auto& child : parent.children()) {
        if (child->isText()) {
            w.u8(kText);
            w.str(collapseWhitespace(child->text()));
        } else if (child->isElement() && !isDroppedElement(*child)) {
            w.u8(kElement);
            w.str(child->tag());
            w.u32(static_cast<uint32_t>(child->attributes().size()));
            for (const Attribute& a : child->attributes()) {
                w.str(a.name);
                w.str(a.value);
            }
            writeChildren(w, *child);
        }
    }
}

bool readChildren(Reader& r, Node& parent, int depth) {
    if (depth > 512) return false;
    const uint32_t n = r.count();
    for (uint32_t i = 0; i < n && r.ok(); ++i) {
        const uint8_t tag = r.u8();
        if (tag == kText) {
            parent.appendChild(Node::text(r.str()));
        } else if (tag == kElement) {
            auto node = Node::element(r.str());
            const uint32_t attrs = r.count();
            for (uint32_t a = 0; a < attrs && r.ok(); ++a) {
                const std::string name = r.str();
                const std::string value = r.str();
                node->setAttribute(name, value);
            }
            Node* added = parent.appendChild(std::move(node));
            if (!readChildren(r, *added, depth + 1)) return false;
        } else {
            return false;
        }
    }
    return r.ok();
}

} // namespace

std::vector<uint8_t> encodeDom(const Node& document) {
    Writer w;
    w.magic(kDomMagic);
    w.u16(kFormatVersion);
    writeChildren(w, document);
    return w.take();
}

std::unique_ptr<Node> decodeDom(const std::vector<uint8_t>& bytes, std::string* error) {
    Reader r(bytes);
    auto document = Node::document();
    if (!r.magic(kDomMagic)) {
        if (error) *error = "not a dom.bin: " + r.error();
        return nullptr;
    }
    const uint16_t version = r.u16();
    if (version != kFormatVersion) {
        if (error) *error = "unsupported dom.bin version";
        return nullptr;
    }
    if (!readChildren(r, *document, 0) || !r.atEnd()) {
        if (error) *error = "corrupt dom.bin: " + (r.error().empty() ? std::string("bad structure") : r.error());
        return nullptr;
    }
    return document;
}

std::vector<uint8_t> encodeStyleSheet(const StyleSheet& sheet) {
    Writer w;
    w.magic(kStyleMagic);
    w.u16(kFormatVersion);
    w.u32(static_cast<uint32_t>(sheet.rules.size()));
    for (const Rule& rule : sheet.rules) {
        w.u32(static_cast<uint32_t>(rule.selectors.size()));
        for (const Selector& sel : rule.selectors) {
            w.i32(sel.specificity);
            w.u32(static_cast<uint32_t>(sel.compounds.size()));
            for (const CompoundSelector& c : sel.compounds) {
                w.str(c.tag);
                w.str(c.id);
                w.u32(static_cast<uint32_t>(c.classes.size()));
                for (const std::string& cls : c.classes) w.str(cls);
            }
            w.u32(static_cast<uint32_t>(sel.combinators.size()));
            for (char comb : sel.combinators) w.u8(static_cast<uint8_t>(comb));
        }
        w.u32(static_cast<uint32_t>(rule.declarations.size()));
        for (const Declaration& d : rule.declarations) {
            w.str(d.property);
            w.str(d.value);
            w.u8(d.important ? 1 : 0);
        }
    }
    return w.take();
}

bool decodeStyleSheet(const std::vector<uint8_t>& bytes, StyleSheet& out, std::string* error) {
    Reader r(bytes);
    StyleSheet sheet;
    if (!r.magic(kStyleMagic)) {
        if (error) *error = "not a style.bin: " + r.error();
        return false;
    }
    if (r.u16() != kFormatVersion) {
        if (error) *error = "unsupported style.bin version";
        return false;
    }

    const uint32_t rules = r.count();
    for (uint32_t i = 0; i < rules && r.ok(); ++i) {
        Rule rule;
        const uint32_t selectors = r.count();
        for (uint32_t s = 0; s < selectors && r.ok(); ++s) {
            Selector sel;
            sel.specificity = r.i32();
            const uint32_t compounds = r.count();
            for (uint32_t c = 0; c < compounds && r.ok(); ++c) {
                CompoundSelector comp;
                comp.tag = r.str();
                comp.id = r.str();
                const uint32_t classes = r.count();
                for (uint32_t k = 0; k < classes && r.ok(); ++k) comp.classes.push_back(r.str());
                sel.compounds.push_back(std::move(comp));
            }
            const uint32_t combinators = r.count();
            for (uint32_t c = 0; c < combinators && r.ok(); ++c) sel.combinators.push_back(static_cast<char>(r.u8()));
            if (r.ok() && (sel.compounds.empty() || sel.combinators.size() + 1 != sel.compounds.size())) {
                if (error) *error = "corrupt style.bin: bad selector";
                return false;
            }
            rule.selectors.push_back(std::move(sel));
        }
        const uint32_t decls = r.count();
        for (uint32_t d = 0; d < decls && r.ok(); ++d) {
            Declaration decl;
            decl.property = r.str();
            decl.value = r.str();
            decl.important = r.u8() != 0;
            rule.declarations.push_back(std::move(decl));
        }
        sheet.rules.push_back(std::move(rule));
    }

    if (!r.ok() || !r.atEnd()) {
        if (error) *error = "corrupt style.bin: " + (r.error().empty() ? std::string("trailing data") : r.error());
        return false;
    }
    out = std::move(sheet);
    return true;
}

} // namespace vpp
