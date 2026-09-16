// HTML parsing. lexbor does the HTML5 tokenising and tree construction; its
// tree is then copied into VPP's own Node structure and discarded, so nothing
// outside this file depends on lexbor.

#include "vpp/html.h"

#include <lexbor/dom/dom.h>
#include <lexbor/html/html.h>

#include <fstream>
#include <sstream>

namespace vpp {

namespace {

std::string toString(const lxb_char_t* data, size_t len) {
    return data ? std::string(reinterpret_cast<const char*>(data), len) : std::string();
}

void convertChildren(lxb_dom_node_t* src, Node& dst) {
    for (lxb_dom_node_t* child = src->first_child; child; child = child->next) {
        switch (child->type) {
        case LXB_DOM_NODE_TYPE_ELEMENT: {
            lxb_dom_element_t* el = lxb_dom_interface_element(child);
            size_t len = 0;
            const lxb_char_t* name = lxb_dom_element_local_name(el, &len);
            auto node = Node::element(toString(name, len));

            for (lxb_dom_attr_t* attr = lxb_dom_element_first_attribute(el); attr;
                 attr = lxb_dom_element_next_attribute(attr)) {
                size_t nameLen = 0;
                size_t valueLen = 0;
                const lxb_char_t* attrName = lxb_dom_attr_local_name(attr, &nameLen);
                const lxb_char_t* attrValue = lxb_dom_attr_value(attr, &valueLen);
                node->setAttribute(toString(attrName, nameLen), toString(attrValue, valueLen));
            }

            Node* added = dst.appendChild(std::move(node));
            convertChildren(child, *added);
            break;
        }
        case LXB_DOM_NODE_TYPE_TEXT: {
            lxb_dom_text_t* text = lxb_dom_interface_text(child);
            dst.appendChild(Node::text(toString(text->char_data.data.data, text->char_data.data.length)));
            break;
        }
        default:
            // Comments, doctype, and processing instructions are dropped.
            break;
        }
    }
}

} // namespace

std::unique_ptr<Node> parseHtml(const std::string& source) {
    auto document = Node::document();

    lxb_html_document_t* doc = lxb_html_document_create();
    if (!doc) return document;

    const lxb_status_t status = lxb_html_document_parse(
        doc, reinterpret_cast<const lxb_char_t*>(source.data()), source.size());
    if (status == LXB_STATUS_OK)
        convertChildren(lxb_dom_interface_node(doc), *document);

    lxb_html_document_destroy(doc);
    return document;
}

namespace {

std::string rawText(const Node& node) {
    std::string text;
    for (const auto& child : node.children())
        if (child->isText()) text += child->text();
    return text;
}

void collectResources(const Node& node, PageResources& out) {
    if (node.isElement()) {
        const std::string& tag = node.tag();
        if (tag == "link") {
            const std::string* rel = node.attribute("rel");
            const std::string* href = node.attribute("href");
            if (rel && href && rel->find("stylesheet") != std::string::npos) out.styles.push_back({true, *href});
            return;
        }
        if (tag == "style") {
            out.styles.push_back({false, rawText(node)});
            return;
        }
        if (tag == "script") {
            if (const std::string* src = node.attribute("src")) out.scripts.push_back({true, *src});
            else out.scripts.push_back({false, rawText(node)});
            return;
        }
    }
    for (const auto& child : node.children())
        collectResources(*child, out);
}

} // namespace

PageResources collectPageResources(const Node& document) {
    PageResources out;
    collectResources(document, out);
    return out;
}

std::unique_ptr<Node> parseHtmlFile(const std::string& path, std::string* error) {
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        if (error) *error = "cannot open " + path;
        return nullptr;
    }
    std::stringstream buffer;
    buffer << in.rdbuf();
    return parseHtml(buffer.str());
}

} // namespace vpp
