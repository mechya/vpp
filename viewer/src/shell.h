#pragma once

// The shell: the viewer's own frame, drawn by the VPP engine above the page.
// Back, forward, reload, the address field, and the window buttons. Pages
// cannot style it; it is a separate document with its own stylesheet.

#include "vpp/canvas.h"
#include "vpp/css.h"
#include "vpp/dom.h"
#include "vpp/font.h"
#include "vpp/layout.h"

#include <memory>
#include <string>

// What a site may ask for in the "window" object of vpp.json.
struct WindowPrefs {
    std::string addressBar = "shown"; // shown | hidden
    bool titleBar = true;             // false hides the whole shell (Ctrl+L still reveals it)
    std::string theme;                // CSS colour for the bar, e.g. "#2563eb"
    float cornerRadius = 0;           // rounded, transparent window corners, CSS px
    int width = 0;                    // initial window width, CSS px; 0 keeps the current size
    int height = 0;                   // initial window height, CSS px; 0 keeps the current size
    bool autoHeight = false;          // "height": "auto": the window follows the page's height
    bool resizable = true;            // defaults to false when the height is automatic
};

WindowPrefs parseWindowPrefs(const std::string& json);

class Shell {
public:
    // Rebuilds the shell document for the given preferences.
    void build(const WindowPrefs& prefs);

    void setAddress(const std::string& text);
    void setNavigation(bool canGoBack, bool canGoForward);
    void setFocused(bool focused);

    // Lays out at the viewport width; height() is valid afterwards.
    void layout(const vpp::FontSet& fonts, float viewportWidth, float scale);
    void paint(vpp::Canvas& canvas, const vpp::FontSet& fonts, float scale) const;

    // Id of the shell control under a CSS-px point ("back", "address", "close", ...), or empty.
    std::string controlAt(float x, float y) const;
    bool contains(float x, float y) const { return visible() && root_ && root_->frame.contains(x, y); }

    bool visible() const { return prefs_.titleBar || forced_; }
    float height() const { return visible() && root_ ? root_->frame.h : 0.0f; }

    // Ctrl+L: show the full shell regardless of preferences until released.
    void force(bool on) { forced_ = on; }
    bool forced() const { return forced_; }
    const WindowPrefs& prefs() const { return prefs_; }

    std::string address;   // the text in the address field
    bool focused = false;  // the address field has keyboard focus
    bool selectAll = false; // the next typed character replaces the whole address

private:
    void refreshAddressText();

    WindowPrefs prefs_;
    bool forced_ = false;
    std::unique_ptr<vpp::Node> document_;
    vpp::StyleSheet sheet_;
    std::unique_ptr<vpp::LayoutBox> root_;
};
