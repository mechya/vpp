#pragma once

#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace vpp {

struct Attribute {
    std::string name;
    std::string value;
};

// VPP's own document tree. The HTML parser produces one of these; everything
// downstream (style, layout, paint, and later the JS bindings) works on it.
class Node {
public:
    enum class Type { Document, Element, Text };

    explicit Node(Type type) : type_(type) {}

    static std::unique_ptr<Node> document();
    static std::unique_ptr<Node> element(std::string tag);
    static std::unique_ptr<Node> text(std::string text);

    Type type() const { return type_; }
    bool isDocument() const { return type_ == Type::Document; }
    bool isElement() const { return type_ == Type::Element; }
    bool isText() const { return type_ == Type::Text; }

    // Lower-case tag name. Empty for non-elements.
    const std::string& tag() const { return tag_; }
    // Raw character data. Empty for non-text nodes.
    const std::string& text() const { return text_; }

    Node* parent() const { return parent_; }
    const std::vector<std::unique_ptr<Node>>& children() const { return children_; }
    Node* appendChild(std::unique_ptr<Node> child);

    // Tree editing, used by the template expander.
    Node* insertChild(size_t index, std::unique_ptr<Node> child);
    std::unique_ptr<Node> detach(Node* child);      // removes child from this node and returns it
    std::vector<std::unique_ptr<Node>> takeChildren();
    size_t indexOf(const Node* child) const;        // npos when not a child
    std::unique_ptr<Node> clone() const;            // deep copy, no parent
    void forEachElement(const std::function<void(Node&)>& fn); // this node (if element) and all descendants

    const std::vector<Attribute>& attributes() const { return attributes_; }
    void setAttribute(std::string name, std::string value);
    void removeAttribute(const std::string& name);
    // Null when the attribute is absent.
    const std::string* attribute(const std::string& name) const;
    std::string id() const;

    // Concatenated text of all descendant text nodes, whitespace collapsed.
    std::string textContent() const;
    // Replaces all children with a single text node.
    void setTextContent(std::string text);
    // Replaces the character data of a text node.
    void setText(std::string text) { text_ = std::move(text); }

    // Depth-first search of this subtree.
    Node* findById(const std::string& id);
    Node* findFirst(const std::string& tag);
    const Node* findFirst(const std::string& tag) const;

    // Nearest ancestor-or-self with the given tag, or null.
    const Node* closest(const std::string& tag) const;

private:
    Type type_;
    std::string tag_;
    std::string text_;
    std::vector<Attribute> attributes_;
    std::vector<std::unique_ptr<Node>> children_;
    Node* parent_ = nullptr;
};

} // namespace vpp
