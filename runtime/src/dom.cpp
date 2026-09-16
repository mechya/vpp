#include "vpp/dom.h"

#include <algorithm>
#include <cctype>

namespace vpp {

namespace {

std::string toLower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

void collectText(const Node& node, std::string& out, bool& pendingSpace) {
    if (node.isText()) {
        for (unsigned char c : node.text()) {
            if (std::isspace(c)) {
                pendingSpace = true;
                continue;
            }
            if (pendingSpace && !out.empty()) out.push_back(' ');
            pendingSpace = false;
            out.push_back(static_cast<char>(c));
        }
        return;
    }
    for (const auto& child : node.children())
        collectText(*child, out, pendingSpace);
}

} // namespace

std::unique_ptr<Node> Node::document() {
    return std::make_unique<Node>(Type::Document);
}

std::unique_ptr<Node> Node::element(std::string tag) {
    auto node = std::make_unique<Node>(Type::Element);
    node->tag_ = toLower(std::move(tag));
    return node;
}

std::unique_ptr<Node> Node::text(std::string text) {
    auto node = std::make_unique<Node>(Type::Text);
    node->text_ = std::move(text);
    return node;
}

Node* Node::appendChild(std::unique_ptr<Node> child) {
    child->parent_ = this;
    children_.push_back(std::move(child));
    return children_.back().get();
}

Node* Node::insertChild(size_t index, std::unique_ptr<Node> child) {
    child->parent_ = this;
    if (index > children_.size()) index = children_.size();
    auto it = children_.insert(children_.begin() + static_cast<std::ptrdiff_t>(index), std::move(child));
    return it->get();
}

std::unique_ptr<Node> Node::detach(Node* child) {
    const size_t index = indexOf(child);
    if (index == std::string::npos) return nullptr;
    std::unique_ptr<Node> out = std::move(children_[index]);
    children_.erase(children_.begin() + static_cast<std::ptrdiff_t>(index));
    out->parent_ = nullptr;
    return out;
}

std::vector<std::unique_ptr<Node>> Node::takeChildren() {
    std::vector<std::unique_ptr<Node>> out = std::move(children_);
    children_.clear();
    for (auto& c : out) c->parent_ = nullptr;
    return out;
}

size_t Node::indexOf(const Node* child) const {
    for (size_t i = 0; i < children_.size(); ++i)
        if (children_[i].get() == child) return i;
    return std::string::npos;
}

std::unique_ptr<Node> Node::clone() const {
    auto copy = std::make_unique<Node>(type_);
    copy->tag_ = tag_;
    copy->text_ = text_;
    copy->attributes_ = attributes_;
    for (const auto& child : children_)
        copy->appendChild(child->clone());
    return copy;
}

void Node::forEachElement(const std::function<void(Node&)>& fn) {
    if (isElement()) fn(*this);
    for (auto& child : children_)
        child->forEachElement(fn);
}

void Node::removeAttribute(const std::string& name) {
    for (size_t i = 0; i < attributes_.size(); ++i) {
        if (attributes_[i].name == name) {
            attributes_.erase(attributes_.begin() + static_cast<std::ptrdiff_t>(i));
            return;
        }
    }
}

void Node::setAttribute(std::string name, std::string value) {
    name = toLower(std::move(name));
    for (Attribute& a : attributes_) {
        if (a.name == name) {
            a.value = std::move(value);
            return;
        }
    }
    attributes_.push_back({std::move(name), std::move(value)});
}

const std::string* Node::attribute(const std::string& name) const {
    for (const Attribute& a : attributes_)
        if (a.name == name) return &a.value;
    return nullptr;
}

std::string Node::id() const {
    const std::string* v = attribute("id");
    return v ? *v : std::string();
}

std::string Node::textContent() const {
    std::string out;
    bool pendingSpace = false;
    collectText(*this, out, pendingSpace);
    return out;
}

void Node::setTextContent(std::string text) {
    children_.clear();
    appendChild(Node::text(std::move(text)));
}

Node* Node::findById(const std::string& id) {
    if (isElement() && this->id() == id) return this;
    for (const auto& child : children_)
        if (Node* found = child->findById(id)) return found;
    return nullptr;
}

Node* Node::findFirst(const std::string& tag) {
    if (isElement() && tag_ == tag) return this;
    for (const auto& child : children_)
        if (Node* found = child->findFirst(tag)) return found;
    return nullptr;
}

const Node* Node::findFirst(const std::string& tag) const {
    return const_cast<Node*>(this)->findFirst(tag);
}

const Node* Node::closest(const std::string& tag) const {
    for (const Node* n = this; n; n = n->parent_)
        if (n->isElement() && n->tag_ == tag) return n;
    return nullptr;
}

} // namespace vpp
