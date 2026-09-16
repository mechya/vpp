#include "vpp/template.h"

#include "vpp/html.h"
#include "vpp/sha256.h"

#include <algorithm>
#include <cctype>
#include <fstream>
#include <set>
#include <sstream>

namespace fs = std::filesystem;

namespace vpp {

namespace {

constexpr int kMaxDepth = 32;

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

std::string trim(const std::string& s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

bool readText(const fs::path& path, std::string& out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) return false;
    std::stringstream buffer;
    buffer << in.rdbuf();
    out = buffer.str();
    return true;
}

bool isNameChar(char c) {
    return std::isalnum(static_cast<unsigned char>(c)) || c == '-' || c == '_' || c == ':';
}

bool isWhitespaceText(const Node& n) {
    if (!n.isText()) return false;
    return std::all_of(n.text().begin(), n.text().end(), [](unsigned char c) { return std::isspace(c); });
}

// Replaces {{ name }} with vars[name]. Unknown names are left untouched in
// text (they may be runtime expressions later) and removed from attribute
// values, so an unset property leaves a clean attribute behind.
std::string substituteText(const std::string& text, const std::map<std::string, std::string>& vars,
                           bool dropUnknown) {
    std::string out;
    size_t i = 0;
    while (i < text.size()) {
        const size_t open = text.find("{{", i);
        if (open == std::string::npos) break;
        const size_t close = text.find("}}", open + 2);
        if (close == std::string::npos) break;
        const std::string key = trim(text.substr(open + 2, close - open - 2));
        auto it = vars.find(key);
        out += text.substr(i, open - i);
        if (it != vars.end()) out += it->second;
        else if (!dropUnknown) out += text.substr(open, close + 2 - open);
        i = close + 2;
    }
    out += text.substr(i);
    return out;
}

std::string sanitizeName(const std::string& s) {
    std::string out;
    for (unsigned char c : s)
        out.push_back(std::isalnum(c) || c == '-' || c == '_' ? static_cast<char>(c) : '-');
    return out.empty() ? "resource" : out;
}

struct Fill {
    std::string slot; // empty = default
    std::string mode; // replace, append, prepend
    std::vector<std::unique_ptr<Node>> nodes;
};

class Expander {
public:
    Expander(const TemplateOptions& options, ExpandedPage& out) : options_(options), out_(out) {}

    bool run(const fs::path& pageFile, std::string* error) {
        std::unique_ptr<Node> document = loadDocument(pageFile, error);
        if (!document) return false;

        // Layouts: a top-level <vpp-layout> in the body wraps the page.
        for (int depth = 0; depth < kMaxDepth; ++depth) {
            Node* layoutTag = findLayoutTag(*document);
            if (!layoutTag) break;
            if (!applyLayout(document, *layoutTag, error)) return false;
        }
        if (findLayoutTag(*document)) return fail(error, pageFile, "layouts nest too deeply");

        if (!expandChildren(*document, 0, error)) return false;
        if (!collectResources(*document, error)) return false;
        out_.document = std::move(document);
        return true;
    }

private:
    static bool fail(std::string* error, const fs::path& file, const std::string& message) {
        if (error) *error = file.string() + ": " + message;
        return false;
    }

    // "/x" is from the project root; anything else is relative to `from`'s directory.
    bool resolve(const std::string& ref, const fs::path& from, fs::path& out, std::string* error) {
        std::error_code ec;
        const fs::path root = fs::weakly_canonical(options_.projectRoot, ec);
        fs::path candidate;
        if (!ref.empty() && (ref[0] == '/' || ref[0] == '\\')) candidate = root / ref.substr(1);
        else candidate = from.parent_path() / ref;
        candidate = fs::weakly_canonical(candidate, ec);

        const std::string r = root.string();
        const std::string c = candidate.string();
        const bool inside = c == r || (c.size() > r.size() && c.compare(0, r.size(), r) == 0 &&
                                       (c[r.size()] == '/' || c[r.size()] == '\\'));
        if (!inside) return fail(error, from, "reference \"" + ref + "\" leaves the project directory");
        out = candidate;
        return true;
    }

    // A project-absolute form ("/styles/global.css") of a resolved path.
    std::string projectPath(const fs::path& resolved) {
        std::error_code ec;
        const fs::path rel = fs::relative(resolved, fs::weakly_canonical(options_.projectRoot, ec), ec);
        std::string s = "/" + rel.generic_string();
        return s;
    }

    // Rewrites file references to project-absolute paths so nodes can be
    // moved between files without losing their origin.
    bool rewriteReferences(Node& root, const fs::path& file, std::string* error) {
        bool ok = true;
        root.forEachElement([&](Node& el) {
            if (!ok) return;
            const std::string& tag = el.tag();
            const char* attr = nullptr;
            if (tag == "link" || tag == "a") attr = "href";
            else if (tag == "script" || tag == "img" || tag == "vpp-include" || tag == "vpp-component" ||
                     tag == "vpp-layout")
                attr = "src";
            if (tag == "a") return; // links between pages stay as written
            if (!attr) return;
            const std::string* value = el.attribute(attr);
            if (!value || value->empty() || value->find("://") != std::string::npos) return;
            fs::path resolved;
            if (!resolve(*value, file, resolved, error)) {
                ok = false;
                return;
            }
            el.setAttribute(attr, projectPath(resolved));
        });
        return ok;
    }

    std::unique_ptr<Node> loadDocument(const fs::path& file, std::string* error) {
        std::string html;
        if (!readText(file, html)) {
            fail(error, file, "cannot read file");
            return nullptr;
        }
        std::unique_ptr<Node> document = parseHtml(preprocessTemplateHtml(html));
        if (!rewriteReferences(*document, file, error)) return nullptr;
        return document;
    }

    // Head children followed by body children, detached from their document.
    bool loadFragment(const fs::path& file, std::vector<std::unique_ptr<Node>>& out, std::string* error) {
        std::unique_ptr<Node> document = loadDocument(file, error);
        if (!document) return false;
        Node* head = document->findFirst("head");
        Node* body = document->findFirst("body");
        if (head)
            for (auto& n : head->takeChildren()) out.push_back(std::move(n));
        if (body)
            for (auto& n : body->takeChildren()) out.push_back(std::move(n));
        return true;
    }

    static Node* findLayoutTag(Node& document) {
        Node* body = document.findFirst("body");
        if (!body) return nullptr;
        for (const auto& child : body->children())
            if (child->isElement() && child->tag() == "vpp-layout") return child.get();
        return nullptr;
    }

    // Collects <vpp-fill> children of `container`; other content forms the default fill.
    static std::vector<Fill> collectFills(Node& container) {
        std::vector<Fill> fills;
        Fill defaultFill;
        defaultFill.mode = "replace";
        for (auto& child : container.takeChildren()) {
            if (child->isElement() && child->tag() == "vpp-fill") {
                Fill f;
                if (const std::string* s = child->attribute("slot")) f.slot = *s;
                f.mode = child->attribute("mode") ? lower(*child->attribute("mode")) : "replace";
                f.nodes = child->takeChildren();
                fills.push_back(std::move(f));
            } else if (!isWhitespaceText(*child)) {
                defaultFill.nodes.push_back(std::move(child));
            }
        }
        if (!defaultFill.nodes.empty()) fills.push_back(std::move(defaultFill));
        return fills;
    }

    static bool isSlot(const Node& n, std::string& name) {
        if (!n.isElement()) return false;
        if (n.tag() == "vpp-slot") {
            name = n.attribute("name") ? *n.attribute("name") : std::string();
            return true;
        }
        if (n.tag() == "meta" && n.attribute("name") && *n.attribute("name") == "vpp-slot") {
            name = n.attribute("content") ? *n.attribute("content") : std::string();
            return true;
        }
        return false;
    }

    static void collectSlots(Node& root, std::vector<Node*>& slots) {
        std::string name;
        for (const auto& child : root.children()) {
            if (isSlot(*child, name)) slots.push_back(child.get());
            else collectSlots(*child, slots);
        }
    }

    // Replaces every slot in `target` with the matching fill, or its fallback content.
    static void fillSlots(Node& target, std::vector<Fill>& fills) {
        std::vector<Node*> slots;
        collectSlots(target, slots);
        for (Node* slot : slots) {
            std::string name;
            isSlot(*slot, name);
            Fill* fill = nullptr;
            for (Fill& f : fills)
                if (f.slot == name) fill = &f;

            Node* parent = slot->parent();
            size_t index = parent->indexOf(slot);
            std::unique_ptr<Node> removed = parent->detach(slot);
            std::vector<std::unique_ptr<Node>> fallback = removed->takeChildren();

            std::vector<std::unique_ptr<Node>> replacement;
            if (!fill) {
                replacement = std::move(fallback);
            } else {
                // Clone so the same fill can serve several slots of one name.
                std::vector<std::unique_ptr<Node>> content;
                for (const auto& n : fill->nodes) content.push_back(n->clone());
                if (fill->mode == "append") {
                    replacement = std::move(fallback);
                    for (auto& n : content) replacement.push_back(std::move(n));
                } else if (fill->mode == "prepend") {
                    replacement = std::move(content);
                    for (auto& n : fallback) replacement.push_back(std::move(n));
                } else {
                    replacement = std::move(content);
                }
            }
            for (auto& n : replacement) parent->insertChild(index++, std::move(n));
        }
    }

    bool applyLayout(std::unique_ptr<Node>& document, Node& layoutTag, std::string* error) {
        const std::string* src = layoutTag.attribute("src");
        if (!src) return fail(error, options_.projectRoot, "<vpp-layout> needs a src attribute");
        fs::path layoutFile;
        if (!resolve(*src, options_.projectRoot / "x", layoutFile, error)) return false;
        std::unique_ptr<Node> layout = loadDocument(layoutFile, error);
        if (!layout) return false;

        std::vector<Fill> fills = collectFills(layoutTag);
        fillSlots(*layout, fills);
        document = std::move(layout);
        return true;
    }

    static void substitute(Node& root, const std::map<std::string, std::string>& vars) {
        if (vars.empty()) return;
        std::function<void(Node&)> visit = [&](Node& n) {
            if (n.isText()) {
                n.setText(substituteText(n.text(), vars, false));
                return;
            }
            if (n.isElement()) {
                std::vector<Attribute> attrs = n.attributes();
                for (const Attribute& a : attrs) {
                    if (a.value.find("{{") == std::string::npos) continue;
                    const std::string v = trim(substituteText(a.value, vars, true));
                    if (v.empty() && trim(a.value).rfind("{{", 0) == 0) n.removeAttribute(a.name);
                    else n.setAttribute(a.name, v);
                }
            }
            for (const auto& c : n.children()) visit(*c);
        };
        visit(root);
    }

    bool expandChildren(Node& parent, int depth, std::string* error) {
        if (depth > kMaxDepth) return fail(error, options_.projectRoot, "includes or components nest too deeply");
        // Iterate by index: expansion replaces children in place.
        for (size_t i = 0; i < parent.children().size();) {
            Node* child = parent.children()[i].get();
            if (!child->isElement()) {
                ++i;
                continue;
            }
            const std::string& tag = child->tag();
            std::vector<std::unique_ptr<Node>> replacement;
            bool replaced = false;

            if (tag == "vpp-include") {
                if (!expandInclude(*child, depth, replacement, error)) return false;
                replaced = true;
            } else if (tag == "vpp-component") {
                const std::string* src = child->attribute("src");
                if (!src) return fail(error, options_.projectRoot, "<vpp-component> needs a src attribute");
                if (!expandComponent(*child, *src, depth, replacement, error)) return false;
                replaced = true;
            } else if (tag.find('-') != std::string::npos && tag.rfind("vpp-", 0) != 0) {
                std::string dir;
                auto alias = options_.componentAliases.find(tag);
                if (alias != options_.componentAliases.end()) dir = alias->second;
                else if (fs::exists(options_.projectRoot / "components" / tag / "component.html")) dir = "/components/" + tag;
                if (!dir.empty()) {
                    if (!expandComponent(*child, dir, depth, replacement, error)) return false;
                    replaced = true;
                }
            }

            if (replaced) {
                parent.detach(child);
                for (auto& n : replacement) parent.insertChild(i++, std::move(n));
            } else {
                if (!expandChildren(*child, depth + 1, error)) return false;
                ++i;
            }
        }
        return true;
    }

    static std::map<std::string, std::string> propsOf(const Node& tag, const std::set<std::string>& skip) {
        std::map<std::string, std::string> props;
        for (const Attribute& a : tag.attributes()) {
            if (skip.count(a.name)) continue;
            props[a.name] = a.value.empty() ? "true" : a.value; // boolean shorthand
        }
        return props;
    }

    bool expandInclude(Node& tag, int depth, std::vector<std::unique_ptr<Node>>& out, std::string* error) {
        const std::string* src = tag.attribute("src");
        if (!src) return fail(error, options_.projectRoot, "<vpp-include> needs a src attribute");
        fs::path file;
        if (!resolve(*src, options_.projectRoot / "x", file, error)) return false;
        if (!fs::exists(file)) {
            if (tag.attribute("optional")) return true; // expands to nothing
            return fail(error, options_.projectRoot, "include not found: " + *src);
        }
        if (!loadFragment(file, out, error)) return false;
        const std::map<std::string, std::string> vars = propsOf(tag, {"src", "optional"});
        for (auto& n : out) {
            substitute(*n, vars);
            if (!expandChildren(*n, depth + 1, error)) return false;
        }
        return true;
    }

    std::string scopeClassFor(const std::string& componentPath) {
        const Sha256Digest d = sha256(reinterpret_cast<const uint8_t*>(componentPath.data()), componentPath.size());
        return "vpp-s" + toHex(d).substr(0, 6);
    }

    bool expandComponent(Node& tag, const std::string& srcRef, int depth, std::vector<std::unique_ptr<Node>>& out,
                         std::string* error) {
        fs::path dir;
        if (!resolve(srcRef, options_.projectRoot / "x", dir, error)) return false;
        const fs::path htmlFile = dir / "component.html";
        if (!fs::exists(htmlFile)) return fail(error, options_.projectRoot, "component not found: " + srcRef);

        std::vector<std::unique_ptr<Node>> nodes;
        if (!loadFragment(htmlFile, nodes, error)) return false;

        // Scoped CSS: every element of the template gets the scope class,
        // and every selector of component.css requires it.
        const std::string key = projectPath(dir);
        const fs::path cssFile = dir / "component.css";
        bool scoped = fs::exists(cssFile);
        std::string json;
        if (scoped && readText(dir / "component.json", json) && json.find("\"scoped\"") != std::string::npos &&
            json.find("false") != std::string::npos)
            scoped = false;
        const std::string scope = scopeClassFor(key);
        if (scoped) {
            for (auto& n : nodes)
                n->forEachElement([&](Node& el) {
                    const std::string* cls = el.attribute("class");
                    el.setAttribute("class", cls && !cls->empty() ? *cls + " " + scope : scope);
                });
        }
        if (fs::exists(cssFile) && !componentStyles_.count(key)) {
            componentStyles_.insert(key);
            std::string css;
            readText(cssFile, css);
            PageStyle style;
            style.name = "component-" + sanitizeName(dir.filename().string());
            style.path = cssFile;
            style.sheet = parseStyleSheet(css);
            if (scoped) {
                for (Rule& rule : style.sheet.rules) {
                    for (Selector& sel : rule.selectors) {
                        sel.compounds.back().classes.push_back(scope);
                        sel.specificity += 100;
                    }
                }
            }
            componentStyleList_.push_back(std::move(style));
        }
        if (fs::exists(dir / "component.js"))
            out_.warnings.push_back(key + "/component.js ignored: component JavaScript is not supported yet");

        // Properties, then slots, then nested templates.
        const std::map<std::string, std::string> props = propsOf(tag, {"src"});
        for (auto& n : nodes) substitute(*n, props);

        std::vector<Fill> fills = collectFills(tag);
        Node holder(Node::Type::Element);
        for (auto& n : nodes) holder.appendChild(std::move(n));
        fillSlots(holder, fills);
        if (!expandChildren(holder, depth + 1, error)) return false;
        out = holder.takeChildren();
        return true;
    }

    bool collectResources(Node& document, std::string* error) {
        int inlineStyles = 0;
        int inlineScripts = 0;
        bool ok = true;
        document.forEachElement([&](Node& el) {
            if (!ok) return;
            const std::string& tag = el.tag();
            if (tag == "link") {
                const std::string* rel = el.attribute("rel");
                const std::string* href = el.attribute("href");
                if (!rel || !href || rel->find("stylesheet") == std::string::npos) return;
                fs::path file;
                if (!resolve(*href, options_.projectRoot / "x", file, error)) { ok = false; return; }
                std::string css;
                if (!readText(file, css)) { ok = fail(error, file, "cannot read stylesheet"); return; }
                PageStyle s;
                s.name = sanitizeName(file.stem().string());
                s.path = file;
                s.sheet = parseStyleSheet(css);
                out_.styles.push_back(std::move(s));
            } else if (tag == "style") {
                std::string css;
                for (const auto& c : el.children()) if (c->isText()) css += c->text();
                PageStyle s;
                s.name = "inline-" + std::to_string(++inlineStyles);
                s.sheet = parseStyleSheet(css);
                out_.styles.push_back(std::move(s));
            } else if (tag == "script") {
                PageScript s;
                if (const std::string* src = el.attribute("src")) {
                    fs::path file;
                    if (!resolve(*src, options_.projectRoot / "x", file, error)) { ok = false; return; }
                    if (!readText(file, s.source)) { ok = fail(error, file, "cannot read script"); return; }
                    s.name = sanitizeName(file.stem().string());
                    s.path = file;
                } else {
                    for (const auto& c : el.children()) if (c->isText()) s.source += c->text();
                    s.name = "inline-" + std::to_string(++inlineScripts);
                }
                out_.scripts.push_back(std::move(s));
            }
        });
        if (!ok) return false;
        for (PageStyle& s : componentStyleList_) out_.styles.push_back(std::move(s));
        componentStyleList_.clear();
        return true;
    }

    const TemplateOptions& options_;
    ExpandedPage& out_;
    std::set<std::string> componentStyles_;
    std::vector<PageStyle> componentStyleList_;
};

} // namespace

std::string preprocessTemplateHtml(const std::string& html) {
    // 1. Self-closing custom tags: <x-y a="1" /> -> <x-y a="1"></x-y>
    std::string out;
    out.reserve(html.size());
    size_t i = 0;
    while (i < html.size()) {
        if (html[i] != '<' || i + 1 >= html.size() || !std::isalpha(static_cast<unsigned char>(html[i + 1]))) {
            out.push_back(html[i++]);
            continue;
        }
        size_t j = i + 1;
        while (j < html.size() && isNameChar(html[j])) ++j;
        const std::string name = lower(html.substr(i + 1, j - i - 1));
        // Scan attributes to the closing '>' respecting quotes.
        size_t k = j;
        char quote = 0;
        while (k < html.size()) {
            const char c = html[k];
            if (quote) {
                if (c == quote) quote = 0;
            } else if (c == '"' || c == '\'') {
                quote = c;
            } else if (c == '>') {
                break;
            }
            ++k;
        }
        if (k >= html.size()) {
            out += html.substr(i);
            break;
        }
        const bool selfClosing = k > i && html[k - 1] == '/';
        if (name.find('-') != std::string::npos && selfClosing) {
            out += html.substr(i, k - 1 - i); // drop the '/'
            out += "></" + name + ">";
        } else {
            out += html.substr(i, k + 1 - i);
        }
        i = k + 1;
    }

    // 2. Slots inside <head>: <vpp-slot name="x"></vpp-slot> -> <meta name="vpp-slot" content="x">
    const std::string lowered = lower(out);
    const size_t headStart = lowered.find("<head");
    const size_t headEnd = lowered.find("</head>");
    if (headStart == std::string::npos || headEnd == std::string::npos || headEnd < headStart) return out;

    std::string head = out.substr(headStart, headEnd - headStart);
    std::string rebuilt;
    size_t p = 0;
    while (p < head.size()) {
        const size_t s = lower(head).find("<vpp-slot", p);
        if (s == std::string::npos) break;
        const size_t gt = head.find('>', s);
        if (gt == std::string::npos) break;
        const std::string tagText = head.substr(s, gt - s);
        std::string name;
        const size_t n = tagText.find("name=");
        if (n != std::string::npos) {
            const char q = tagText[n + 5];
            const size_t q2 = tagText.find(q, n + 6);
            if (q2 != std::string::npos) name = tagText.substr(n + 6, q2 - n - 6);
        }
        rebuilt += head.substr(p, s - p);
        rebuilt += "<meta name=\"vpp-slot\" content=\"" + name + "\">";
        p = gt + 1;
        const size_t closeTag = lower(head).find("</vpp-slot>", p);
        if (closeTag != std::string::npos) p = closeTag + 11;
    }
    rebuilt += head.substr(p);
    return out.substr(0, headStart) + rebuilt + out.substr(headEnd);
}

bool expandPage(const fs::path& pageFile, const TemplateOptions& options, ExpandedPage& out, std::string* error) {
    out = ExpandedPage{};
    Expander expander(options, out);
    return expander.run(pageFile, error);
}

fs::path findProjectRoot(const fs::path& file) {
    std::error_code ec;
    fs::path dir = fs::absolute(file, ec).parent_path();
    for (fs::path p = dir; !p.empty(); p = p.parent_path()) {
        if (fs::exists(p / "vpp.json")) return p;
        if (p == p.root_path()) break;
    }
    return dir;
}

std::map<std::string, std::string> parseComponentAliases(const std::string& json) {
    // Minimal: find "components": { "tag": "/path", ... }
    std::map<std::string, std::string> out;
    const size_t key = json.find("\"components\"");
    if (key == std::string::npos) return out;
    const size_t open = json.find('{', key);
    const size_t close = json.find('}', open == std::string::npos ? key : open);
    if (open == std::string::npos || close == std::string::npos) return out;
    const std::string body = json.substr(open + 1, close - open - 1);
    size_t i = 0;
    while (true) {
        const size_t q1 = body.find('"', i);
        if (q1 == std::string::npos) break;
        const size_t q2 = body.find('"', q1 + 1);
        const size_t q3 = body.find('"', q2 + 1);
        const size_t q4 = body.find('"', q3 + 1);
        if (q2 == std::string::npos || q3 == std::string::npos || q4 == std::string::npos) break;
        out[lower(body.substr(q1 + 1, q2 - q1 - 1))] = body.substr(q3 + 1, q4 - q3 - 1);
        i = q4 + 1;
    }
    return out;
}

} // namespace vpp
