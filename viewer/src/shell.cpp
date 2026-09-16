#include "shell.h"

#include "vpp/html.h"
#include "vpp/paint.h"

#include <cctype>
#include <cstdlib>

namespace {

// The shell page. ASCII only: the text renderer does not shape other
// scripts yet. Colours that depend on the theme are filled in by build().
const char* kShellHtml = R"(
<html><body>
<div id="bar">
  <button id="back" class="nav">&lt;</button>
  <button id="forward" class="nav">&gt;</button>
  <button id="reload" class="nav">R</button>
  <div id="address">
    <div id="address-text"></div>
  </div>
  <button id="minimize" class="win">_</button>
  <button id="maximize" class="win">[ ]</button>
  <button id="close" class="win close">X</button>
</div>
</body></html>
)";

const char* kShellCss = R"(
body { margin: 0; padding: 0; }
#bar { display: flex; align-items: center; gap: 6px; padding: 6px 8px; background-color: THEME; }
button.nav, button.win {
  padding: 4px 10px; font-size: 13px; border-radius: 6px;
  background-color: BUTTON; color: TEXT; font-weight: bold;
}
button.disabled { color: MUTED; }
button.close { background-color: #dc2626; color: white; }
#address { flex: 1; padding: 5px 10px; background-color: FIELD; border: 2px solid FIELD; border-radius: 6px; }
#address.focused { border: 2px solid #2563eb; }
#address-text { font-size: 13px; color: #18181b; }
#address.hidden { display: none; }
)";

std::string replaceAll(std::string s, const std::string& from, const std::string& to) {
    size_t pos = 0;
    while ((pos = s.find(from, pos)) != std::string::npos) {
        s.replace(pos, from.size(), to);
        pos += to.size();
    }
    return s;
}

// Relative luminance of a #rrggbb colour, 0..1; 1 (light) if unparseable.
float luminance(const std::string& hex) {
    if (hex.size() != 7 || hex[0] != '#') return 1.0f;
    const long v = std::strtol(hex.c_str() + 1, nullptr, 16);
    const float r = static_cast<float>((v >> 16) & 255) / 255.0f;
    const float g = static_cast<float>((v >> 8) & 255) / 255.0f;
    const float b = static_cast<float>(v & 255) / 255.0f;
    return 0.2126f * r + 0.7152f * g + 0.0722f * b;
}

std::string jsonString(const std::string& json, const std::string& key) {
    const size_t k = json.find("\"" + key + "\"");
    if (k == std::string::npos) return std::string();
    const size_t colon = json.find(':', k);
    if (colon == std::string::npos) return std::string();
    size_t i = colon + 1;
    while (i < json.size() && std::isspace(static_cast<unsigned char>(json[i]))) ++i;
    if (i < json.size() && json[i] == '"') {
        const size_t end = json.find('"', i + 1);
        return end == std::string::npos ? std::string() : json.substr(i + 1, end - i - 1);
    }
    size_t end = i;
    while (end < json.size() && (std::isalnum(static_cast<unsigned char>(json[end])) || json[end] == '.')) ++end;
    return json.substr(i, end - i);
}

} // namespace

WindowPrefs parseWindowPrefs(const std::string& json) {
    WindowPrefs prefs;
    if (json.empty()) return prefs;
    const std::string bar = jsonString(json, "addressBar");
    if (bar == "hidden" || bar == "shown") prefs.addressBar = bar;
    if (jsonString(json, "titleBar") == "false") prefs.titleBar = false;
    const std::string theme = jsonString(json, "theme");
    if (!theme.empty()) prefs.theme = theme;

    const std::string radius = jsonString(json, "cornerRadius");
    if (!radius.empty()) prefs.cornerRadius = std::strtof(radius.c_str(), nullptr);
    const std::string width = jsonString(json, "width");
    if (!width.empty()) prefs.width = std::atoi(width.c_str());
    const std::string height = jsonString(json, "height");
    if (height == "auto") prefs.autoHeight = true;
    else if (!height.empty()) prefs.height = std::atoi(height.c_str());

    const std::string resizable = jsonString(json, "resizable");
    if (resizable == "true") prefs.resizable = true;
    else if (resizable == "false") prefs.resizable = false;
    else prefs.resizable = !prefs.autoHeight;
    return prefs;
}

void Shell::build(const WindowPrefs& prefs) {
    prefs_ = prefs;
    const std::string theme = prefs.theme.empty() ? "#27272a" : prefs.theme;
    const bool dark = luminance(theme) < 0.5f;

    std::string css = kShellCss;
    css = replaceAll(css, "THEME", theme);
    css = replaceAll(css, "BUTTON", dark ? "rgba(255,255,255,0.14)" : "rgba(0,0,0,0.08)");
    css = replaceAll(css, "TEXT", dark ? "#ffffff" : "#18181b");
    css = replaceAll(css, "MUTED", dark ? "rgba(255,255,255,0.35)" : "rgba(0,0,0,0.3)");
    css = replaceAll(css, "FIELD", dark ? "#f4f4f5" : "#ffffff");

    document_ = vpp::parseHtml(kShellHtml);
    sheet_ = vpp::parseStyleSheet(css);
    root_.reset();

    if (prefs.addressBar == "hidden")
        if (vpp::Node* field = document_->findById("address")) field->setAttribute("class", "hidden");
    refreshAddressText();
}

void Shell::refreshAddressText() {
    if (!document_) return;
    if (vpp::Node* text = document_->findById("address-text")) {
        std::string shown = address.empty() && !focused ? "Enter a page address: a .vpp file or URL" : address;
        if (focused) shown += "|";
        text->setTextContent(shown);
    }
}

void Shell::setAddress(const std::string& text) {
    address = text;
    refreshAddressText();
}

void Shell::setNavigation(bool canGoBack, bool canGoForward) {
    if (!document_) return;
    if (vpp::Node* b = document_->findById("back")) b->setAttribute("class", canGoBack ? "nav" : "nav disabled");
    if (vpp::Node* f = document_->findById("forward")) f->setAttribute("class", canGoForward ? "nav" : "nav disabled");
}

void Shell::setFocused(bool on) {
    focused = on;
    if (!on) selectAll = false;
    if (!document_) return;
    if (vpp::Node* field = document_->findById("address")) {
        if (prefs_.addressBar != "hidden" || forced_) field->setAttribute("class", on ? "focused" : "");
    }
    refreshAddressText();
}

void Shell::layout(const vpp::FontSet& fonts, float viewportWidth, float scale) {
    if (!document_ || !visible()) {
        root_.reset();
        return;
    }
    // A forced reveal shows the address field even when the site hid it.
    if (vpp::Node* field = document_->findById("address")) {
        if (prefs_.addressBar == "hidden") field->setAttribute("class", forced_ ? (focused ? "focused" : "") : "hidden");
    }
    const vpp::LayoutContext ctx{fonts, scale, &sheet_};
    root_ = vpp::layoutDocument(*document_, ctx, viewportWidth);
}

void Shell::paint(vpp::Canvas& canvas, const vpp::FontSet& fonts, float scale) const {
    if (root_) vpp::paint(*root_, canvas, fonts, scale);
}

std::string Shell::controlAt(float x, float y) const {
    if (!root_) return std::string();
    const vpp::LayoutBox* box = vpp::hitTest(*root_, x, y);
    for (const vpp::Node* n = box ? box->node : nullptr; n; n = n->parent()) {
        if (!n->isElement()) continue;
        const std::string id = n->id();
        if (id == "back" || id == "forward" || id == "reload" || id == "minimize" || id == "maximize" ||
            id == "close" || id == "address")
            return id;
    }
    return std::string();
}
