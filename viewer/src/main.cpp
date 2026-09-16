// VPP Viewer.
//
//   vpp_viewer [page.html | dist/<page> | page.vpp | https://host/site/page.vpp] [--allow-unsigned]
//
// Development mode takes a page's HTML file: the page is expanded with its
// layout, includes, and components, and its stylesheets and scripts are
// loaded as text.
//
// Release mode takes a compiled page: a dist/<page> directory, a .vpp
// package, or a URL. Packages must carry a valid publisher signature
// (--allow-unsigned relaxes this for development), the publisher key is
// pinned per site id on first use, and every resource is hash verified
// before anything runs. A URL installs or updates the page in the local
// store first, downloading only resources not already present.
//
// Links between pages (<a href="about.vpp">) navigate within the same
// window. Alt+Left / Alt+Right and the mouse back/forward buttons move
// through history. R reloads the current page.

#include "vpp/binary.h"
#include "vpp/canvas.h"
#include "vpp/css.h"
#include "vpp/dom.h"
#include "vpp/font.h"
#include "vpp/html.h"
#include "vpp/http.h"
#include "vpp/layout.h"
#include "vpp/package.h"
#include "vpp/paint.h"
#include "vpp/script.h"
#include "vpp/template.h"
#include "vpp/updater.h"

#include "shell.h"

#include <SDL3/SDL.h>
#include <SDL3/SDL_main.h>

#include <algorithm>
#include <cctype>
#include <cmath>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <memory>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace {

constexpr int kLogicalWidth = 800;
constexpr int kLogicalHeight = 520;
constexpr float kDragStripHeight = 40.0f; // CSS px, top band that moves the frameless window
constexpr float kUiTextSize = 18.0f;
constexpr float kResizeBorder = 6.0f; // CSS px, frameless-window resize grip

const vpp::Color kBackground = vpp::Color::rgb(244, 244, 245);
const vpp::Color kTextDark = vpp::Color::rgb(24, 24, 27);
const vpp::Color kTextLight = vpp::Color::rgb(255, 255, 255);
const vpp::Color kAccent = vpp::Color::rgb(37, 99, 235);
const vpp::Color kAccentDown = vpp::Color::rgb(29, 78, 216);
const vpp::Color kCard = vpp::Color::rgb(255, 255, 255);
const vpp::Color kOverlay = vpp::Color::rgba(0, 0, 0, 110);
const vpp::Color kShadow = vpp::Color::rgba(0, 0, 0, 40);

struct Popup {
    bool visible = false;
    std::string message;
    vpp::Rect rect;
    vpp::Rect ok;
    bool okPressed = false;
};

struct App {
    SDL_Window* window = nullptr;
    SDL_Renderer* renderer = nullptr;
    SDL_Texture* texture = nullptr;

    vpp::Canvas canvas{kLogicalWidth, kLogicalHeight};
    vpp::FontSet fonts;
    float scale = 1.0f; // device px per CSS px

    std::string location; // current page: HTML path, dist directory, .vpp path, URL, or empty for the start page
    std::vector<std::string> history;
    size_t historyIndex = 0;
    bool allowUnsigned = false;
    std::string title = "VPP Viewer";

    Shell shell;
    WindowPrefs pagePrefs;    // what the current page asked for
    std::string siteId;       // of the current page, to notice site changes
    std::string shownSiteId;  // the site the shell was last built for

    std::unique_ptr<vpp::Node> document;
    vpp::StyleSheet sheet;
    std::unique_ptr<vpp::LayoutBox> layoutRoot;
    const vpp::Node* pressedNode = nullptr;

    Popup popup;
    bool needsLayout = false;
    bool dirty = true;
    bool running = true;

    // Declared last so it is destroyed before the document it points into.
    std::unique_ptr<vpp::ScriptHost> script;
};

int px(float v, float scale) {
    return static_cast<int>(std::lround(v * scale));
}

bool readFile(const fs::path& path, std::string& out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) return false;
    std::stringstream buffer;
    buffer << in.rdbuf();
    out = buffer.str();
    return true;
}

bool readBytes(const fs::path& path, std::vector<uint8_t>& out) {
    std::string text;
    if (!readFile(path, text)) return false;
    out.assign(text.begin(), text.end());
    return true;
}

std::string lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

bool endsWith(const std::string& s, const std::string& suffix) {
    return s.size() >= suffix.size() && s.compare(s.size() - suffix.size(), suffix.size(), suffix) == 0;
}

bool loadFonts(vpp::FontSet& fonts) {
    struct Pair {
        const char* regular;
        const char* bold;
    };
    const Pair candidates[] = {
        {"C:/Windows/Fonts/segoeui.ttf", "C:/Windows/Fonts/segoeuib.ttf"},
        {"C:/Windows/Fonts/arial.ttf", "C:/Windows/Fonts/arialbd.ttf"},
        {"/System/Library/Fonts/Supplemental/Arial.ttf", "/System/Library/Fonts/Supplemental/Arial Bold.ttf"},
        {"/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"},
    };
    for (const Pair& c : candidates) {
        if (fonts.regular.load(c.regular)) {
            fonts.bold.load(c.bold);
            std::printf("font: %s (bold %s)\n", c.regular, fonts.bold.loaded() ? "yes" : "no");
            return true;
        }
    }
    std::printf("font: none found, text will not render\n");
    return false;
}

// --- page loading ----------------------------------------------------------------

std::unique_ptr<vpp::Node> errorPage(const std::string& title, const std::string& detail) {
    return vpp::parseHtml("<html><body><h1>" + title + "</h1><p>" + detail + "</p></body></html>");
}

void createScriptHost(App& app) {
    vpp::ScriptCallbacks cb;
    cb.log = [](const std::string& line) {
        std::printf("console: %s\n", line.c_str());
        std::fflush(stdout);
    };
    cb.popup = [&app](const std::string& message) {
        app.popup.message = message;
        app.popup.visible = true;
        app.popup.okPressed = false;
        app.dirty = true;
    };
    cb.close = [&app] { app.running = false; };
    cb.minimize = [&app] { SDL_MinimizeWindow(app.window); };
    cb.maximize = [&app] {
        if (SDL_GetWindowFlags(app.window) & SDL_WINDOW_MAXIMIZED) SDL_RestoreWindow(app.window);
        else SDL_MaximizeWindow(app.window);
    };
    cb.invalidate = [&app] {
        app.needsLayout = true;
        app.dirty = true;
    };
    app.script = std::make_unique<vpp::ScriptHost>(*app.document, std::move(cb));
}

void loadStart(App& app) {
    std::printf("mode: start\n");
    app.document = vpp::parseHtml(
        "<html><body><h1>VPP Viewer</h1>"
        "<p>Enter the address of a page in the bar above: a .vpp URL such as "
        "https://example.com/site/home.vpp, or a local .vpp file or page.html.</p>"
        "<p>Press Ctrl+L to focus the address bar at any time. Alt+Left and Alt+Right move through history.</p>"
        "</body></html>");
    app.title = "VPP Viewer";
    createScriptHost(app);
}

void showError(App& app, const std::string& title, const std::string& detail) {
    std::printf("page error: %s\n", detail.c_str());
    app.document = errorPage(title, detail);
    app.title = title;
    createScriptHost(app);
}

// Development: expand the page with its templates; parse everything as text.
void loadDevelopment(App& app, const fs::path& page) {
    std::printf("mode: development\npage: %s\n", page.string().c_str());

    vpp::TemplateOptions options;
    options.projectRoot = vpp::findProjectRoot(page);
    std::string json;
    app.siteId = "dev:" + options.projectRoot.u8string();
    if (readFile(options.projectRoot / "vpp.json", json)) {
        options.componentAliases = vpp::parseComponentAliases(json);
        vpp::AppManifest manifest;
        if (vpp::parseAppManifest(json, manifest, nullptr)) {
            app.pagePrefs = parseWindowPrefs(manifest.window);
            app.siteId = manifest.id;
        }
    }

    vpp::ExpandedPage expanded;
    std::string error;
    if (!vpp::expandPage(page, options, expanded, &error)) {
        showError(app, "Could not build page", error);
        return;
    }
    for (const std::string& w : expanded.warnings) std::printf("warning: %s\n", w.c_str());

    app.document = std::move(expanded.document);
    for (vpp::PageStyle& s : expanded.styles) {
        std::printf("style: %s (%zu rules)\n", s.name.c_str(), s.sheet.rules.size());
        app.sheet.append(std::move(s.sheet));
    }
    app.title = page.stem().string();
    if (const vpp::Node* t = app.document->findFirst("title")) app.title = t->textContent();

    createScriptHost(app);
    for (const vpp::PageScript& s : expanded.scripts) {
        if (app.script->runSource(s.source, s.path.empty() ? s.name : s.path.filename().string(), &error))
            std::printf("script: %s (source)\n", s.name.c_str());
        else
            std::printf("script error: %s\n%s\n", s.name.c_str(), error.c_str());
    }
}

// Release: build the page from compiled resources. fetch(name, out, error)
// supplies each; names lists everything available, sorted.
void loadResources(App& app, const std::string& label, const std::vector<std::string>& names,
                   const std::function<bool(const std::string&, std::vector<uint8_t>&, std::string*)>& fetch) {
    std::string error;
    std::vector<uint8_t> bytes;

    if (!fetch("dom.bin", bytes, &error)) {
        showError(app, "Could not open page", error);
        return;
    }
    app.document = vpp::decodeDom(bytes, &error);
    if (!app.document) {
        showError(app, "Invalid page", error);
        return;
    }
    std::printf("page: %s (dom.bin, %zu bytes)\n", label.c_str(), bytes.size());
    if (const vpp::Node* t = app.document->findFirst("title")) app.title = t->textContent();

    for (const std::string& name : names) {
        if (!(name.rfind("style/", 0) == 0 || name == "style.bin")) continue;
        if (!fetch(name, bytes, &error)) continue;
        vpp::StyleSheet part;
        if (vpp::decodeStyleSheet(bytes, part, &error)) {
            std::printf("style: %s (%zu rules, %zu bytes)\n", name.c_str(), part.rules.size(), bytes.size());
            app.sheet.append(std::move(part));
        } else {
            std::printf("style error: %s: %s\n", name.c_str(), error.c_str());
        }
    }

    createScriptHost(app);
    for (const std::string& name : names) {
        if (!(name.rfind("code/", 0) == 0 || name == "code.bin")) continue;
        if (!fetch(name, bytes, &error)) continue;
        if (app.script->runBytecode(bytes, &error))
            std::printf("script: %s (bytecode, %zu bytes)\n", name.c_str(), bytes.size());
        else
            std::printf("script error: %s\n%s\n", name.c_str(), error.c_str());
    }
}

void loadReleaseDirectory(App& app, const fs::path& dir) {
    std::printf("mode: release (directory)\n");
    std::vector<std::string> names;
    std::error_code ec;
    for (const auto& entry : fs::recursive_directory_iterator(dir, ec))
        if (entry.is_regular_file()) names.push_back(fs::relative(entry.path(), dir, ec).generic_string());
    std::sort(names.begin(), names.end());
    app.title = dir.filename().string();
    loadResources(app, dir.string(), names, [&dir](const std::string& name, std::vector<uint8_t>& out, std::string* error) {
        if (readBytes(dir / name, out)) return true;
        *error = "cannot read " + (dir / name).string();
        return false;
    });
}

// --- publisher trust: the key seen first for a site id is remembered ----------

fs::path prefDir(const char* sub) {
    char* pref = SDL_GetPrefPath("VPP", "Viewer");
    fs::path dir = pref ? fs::u8path(pref) / sub : fs::temp_directory_path() / (std::string("vpp-") + sub);
    if (pref) SDL_free(pref);
    std::error_code ec;
    fs::create_directories(dir, ec);
    return dir;
}

bool checkPublisherTrust(const std::string& siteId, const vpp::PublicKey& key, std::string* error) {
    const fs::path file = prefDir("trust") / (vpp::sanitizeId(siteId) + ".pub");
    const std::string hex = vpp::hexEncode(key.data(), key.size());

    std::string known;
    if (readFile(file, known)) {
        while (!known.empty() && std::isspace(static_cast<unsigned char>(known.back()))) known.pop_back();
        if (known == hex) {
            std::printf("publisher: %s (known)\n", hex.c_str());
            return true;
        }
        *error = "publisher key changed for " + siteId + ": expected " + known.substr(0, 16) + "..., got " +
                 hex.substr(0, 16) + "... Delete " + file.string() + " if this is intended.";
        return false;
    }
    std::ofstream out(file, std::ios::trunc);
    out << hex << "\n";
    std::printf("publisher: %s (trusted on first use, recorded in %s)\n", hex.c_str(), file.string().c_str());
    return true;
}

void loadPackage(App& app, const fs::path& file) {
    std::printf("mode: release (package)\n");
    std::vector<uint8_t> bytes;
    if (!readBytes(file, bytes)) {
        showError(app, "Could not open package", "cannot read " + file.string());
        return;
    }

    vpp::Package pkg;
    std::string error;
    if (!vpp::Package::open(std::move(bytes), pkg, &error)) {
        showError(app, "Invalid package", error);
        return;
    }

    const vpp::AppManifest& m = pkg.manifest();
    std::printf("package: %s / %s %s (%s), %zu resources\n", m.name.c_str(), m.page.c_str(), m.version.c_str(),
                m.id.c_str(), pkg.entries().size());
    app.siteId = m.id;
    app.pagePrefs = parseWindowPrefs(m.window);

    // 1. Signature. Nothing below runs on an unsigned or tampered package.
    switch (pkg.verifySignature()) {
    case vpp::SignatureStatus::Invalid:
        showError(app, "Package rejected", "signature invalid: the package was modified after it was signed");
        return;
    case vpp::SignatureStatus::Unsigned:
        if (!app.allowUnsigned) {
            showError(app, "Package rejected",
                      "unsigned package. Sign it with vpppack --key, or start the viewer with --allow-unsigned for development.");
            return;
        }
        std::printf("warning: unsigned package accepted because of --allow-unsigned\n");
        break;
    case vpp::SignatureStatus::Valid:
        std::printf("signature: valid\n");
        if (!checkPublisherTrust(m.id, pkg.publisherKey(), &error)) {
            showError(app, "Package rejected", error);
            return;
        }
        break;
    }

    // 2. Resource hashes. One bad resource rejects the whole package.
    std::vector<std::string> names;
    for (const vpp::PackageEntry& e : pkg.entries()) {
        std::vector<uint8_t> data;
        if (!pkg.read(e.name, data, &error)) {
            showError(app, "Package rejected", error);
            return;
        }
        names.push_back(e.name);
    }
    std::sort(names.begin(), names.end());
    std::printf("verified: %zu resources\n", names.size());

    app.title = m.name + (m.page.empty() ? "" : " / " + m.page);
    loadResources(app, file.filename().string(), names, [&pkg](const std::string& name, std::vector<uint8_t>& out, std::string* error) {
        if (!pkg.find(name)) {
            *error = "package has no " + name;
            return false;
        }
        return pkg.read(name, out, error);
    });
}

void loadRemote(App& app, const std::string& url) {
    std::printf("mode: remote\nurl: %s\n", url.c_str());
    vpp::AppStore store(prefDir("apps"));
    vpp::SyncResult result;
    std::string error;
    const bool ok = store.sync(url, vpp::httpGet, result, &error, [](const std::string& line) {
        std::printf("update: %s\n", line.c_str());
    });
    if (!ok) {
        showError(app, "Could not fetch page", error);
        return;
    }
    loadPackage(app, result.packagePath);
}

// Rebuilds the shell for the page just loaded. Crossing from one site into
// another reveals the full shell, whatever the new site asked for, until
// Esc. A site opened directly starts the way it asked; Ctrl+L still works.
void applyShell(App& app) {
    const bool crossedSites = !app.shownSiteId.empty() && app.siteId != app.shownSiteId;
    app.shownSiteId = app.siteId;
    if (crossedSites && (!app.pagePrefs.titleBar || app.pagePrefs.addressBar == "hidden")) app.shell.force(true);
    app.shell.build(app.pagePrefs);
    app.shell.setAddress(app.location);
    app.shell.setNavigation(app.historyIndex > 0, app.historyIndex + 1 < app.history.size());
    app.shell.setFocused(false);
    if (app.window) {
        SDL_StopTextInput(app.window);
        SDL_SetWindowResizable(app.window, app.pagePrefs.resizable);
        if (app.pagePrefs.width > 0 || app.pagePrefs.height > 0) {
            int w = 0, h = 0;
            SDL_GetWindowSize(app.window, &w, &h);
            int pw = 0, ph = 0;
            SDL_GetWindowSizeInPixels(app.window, &pw, &ph);
            const float unitsPerPixel = pw > 0 ? static_cast<float>(w) / pw : 1.0f;
            const int nw = app.pagePrefs.width > 0 ? px(app.pagePrefs.width * app.scale * unitsPerPixel, 1) : w;
            const int nh = app.pagePrefs.height > 0 ? px(app.pagePrefs.height * app.scale * unitsPerPixel, 1) : h;
            if (nw != w || nh != h) SDL_SetWindowSize(app.window, nw, nh);
        }
    }
}

// "height": "auto": the window wraps the page. Called after every layout.
void fitWindowToPage(App& app) {
    if (!app.pagePrefs.autoHeight || !app.window || !app.layoutRoot) return;
    if (SDL_GetWindowFlags(app.window) & SDL_WINDOW_MAXIMIZED) return;
    const float contentCss = app.shell.height() + app.layoutRoot->frame.h;
    const int wantPx = std::max(px(contentCss, app.scale), 80);
    if (std::abs(wantPx - app.canvas.height()) <= 1) return;

    int w = 0, h = 0;
    SDL_GetWindowSize(app.window, &w, &h);
    int pw = 0, ph = 0;
    SDL_GetWindowSizeInPixels(app.window, &pw, &ph);
    const float unitsPerPixel = ph > 0 ? static_cast<float>(h) / ph : 1.0f;
    SDL_SetWindowSize(app.window, w, px(wantPx * unitsPerPixel, 1));
}

void loadPage(App& app) {
    app.script.reset();
    app.popup.visible = false;
    app.sheet = vpp::StyleSheet{};
    app.title = "VPP Viewer";
    app.pagePrefs = WindowPrefs{};
    app.siteId.clear();

    if (app.location.empty()) {
        loadStart(app);
    } else if (vpp::isRemoteUrl(app.location)) {
        loadRemote(app, app.location);
    } else {
        // SDL hands main() UTF-8 arguments on every platform, so decode as
        // UTF-8 rather than the system code page.
        const fs::path path = fs::u8path(app.location);
        const std::string ext = lower(path.extension().string());
        std::error_code ec;
        if (fs::is_directory(path, ec)) loadReleaseDirectory(app, path);
        else if (ext == ".vpp") loadPackage(app, path);
        else if (ext == ".bin") loadReleaseDirectory(app, path.parent_path());
        else loadDevelopment(app, path);
    }
    if (app.window) SDL_SetWindowTitle(app.window, app.title.c_str());
    applyShell(app);
    std::fflush(stdout);
}

// --- navigation ---------------------------------------------------------------------

// Resolves a link's href against the current location. Empty when not navigable.
std::string resolveLink(const App& app, const std::string& href) {
    if (href.empty() || href[0] == '#') return std::string();
    if (vpp::isRemoteUrl(href)) return href;

    const std::string& cur = app.location;
    if (vpp::isRemoteUrl(cur)) {
        const size_t slash = cur.find_last_of('/');
        return cur.substr(0, slash + 1) + href;
    }

    const fs::path p = fs::u8path(cur);
    const std::string ext = lower(p.extension().string());
    const std::string target = fs::u8path(href).stem().string();
    std::error_code ec;

    if (ext == ".vpp") return (p.parent_path() / href).u8string();
    if (ext == ".html" || ext == ".htm") {
        // Development: a link to about.vpp means the page source about.html.
        const std::string file = endsWith(lower(href), ".vpp") ? target + ".html" : href;
        return (p.parent_path() / file).u8string();
    }
    // A dist/<page> directory (or its dom.bin): sibling directory dist/<target>.
    const fs::path dir = fs::is_directory(p, ec) ? p : p.parent_path();
    return (dir.parent_path() / target).u8string();
}

void relayout(App& app);

void navigate(App& app, const std::string& location, bool push) {
    if (push) {
        if (app.historyIndex + 1 < app.history.size())
            app.history.erase(app.history.begin() + static_cast<std::ptrdiff_t>(app.historyIndex + 1), app.history.end());
        app.history.push_back(location);
        app.historyIndex = app.history.size() - 1;
    }
    app.location = location;
    loadPage(app);
    relayout(app);
}

void goBack(App& app) {
    if (app.historyIndex == 0) return;
    --app.historyIndex;
    navigate(app, app.history[app.historyIndex], false);
}

void goForward(App& app) {
    if (app.historyIndex + 1 >= app.history.size()) return;
    ++app.historyIndex;
    navigate(app, app.history[app.historyIndex], false);
}

// --- layout and rendering ----------------------------------------------------------

void relayout(App& app) {
    const float viewportWidth = app.canvas.width() / app.scale;
    app.shell.layout(app.fonts, viewportWidth, app.scale);

    const vpp::LayoutContext ctx{app.fonts, app.scale, &app.sheet};
    app.layoutRoot = app.document ? vpp::layoutDocument(*app.document, ctx, viewportWidth) : nullptr;
    // The page sits below the shell in one coordinate space, so hit testing
    // and painting need no special cases.
    if (app.layoutRoot) vpp::translateLayout(*app.layoutRoot, 0, app.shell.height());
    app.needsLayout = false;
    app.dirty = true;
    fitWindowToPage(app);
}

// The canvas takes the background of <html>, else <body>, as in a browser.
vpp::Color pageBackground(const App& app) {
    if (!app.layoutRoot) return kBackground;
    if (app.layoutRoot->style.hasBackground) return app.layoutRoot->style.background;
    for (const auto& child : app.layoutRoot->children)
        if (child->node && child->node->tag() == "body" && child->style.hasBackground) return child->style.background;
    return kBackground;
}

void layoutPopup(App& app) {
    const int w = px(340, app.scale);
    const int h = px(180, app.scale);
    app.popup.rect = {(app.canvas.width() - w) / 2, (app.canvas.height() - h) / 2, w, h};
    app.popup.ok = {app.popup.rect.x + w - px(110, app.scale), app.popup.rect.y + h - px(60, app.scale),
                    px(90, app.scale), px(40, app.scale)};
}

void drawCenteredText(App& app, const vpp::Rect& box, const std::string& text, vpp::Color color) {
    if (!app.fonts.loaded()) return;
    const float size = kUiTextSize * app.scale;
    const vpp::Font& font = app.fonts.regular;
    const int textW = font.measure(text, size);
    const int x = box.x + (box.w - textW) / 2;
    const int baseline = box.y + static_cast<int>((box.h - font.lineHeight(size)) / 2.0f + font.ascent(size) + 0.5f);
    font.draw(app.canvas, x, baseline, text, size, color);
}

void drawPopup(App& app) {
    app.canvas.fillRect({0, 0, app.canvas.width(), app.canvas.height()}, kOverlay);

    vpp::Rect shadow = app.popup.rect;
    shadow.y += px(6, app.scale);
    app.canvas.fillRoundRect(shadow, px(12, app.scale), kShadow);
    app.canvas.fillRoundRect(app.popup.rect, px(12, app.scale), kCard);

    vpp::Rect textBox = app.popup.rect;
    textBox.h -= px(60, app.scale);
    drawCenteredText(app, textBox, app.popup.message, kTextDark);

    app.canvas.fillRoundRect(app.popup.ok, px(8, app.scale), app.popup.okPressed ? kAccentDown : kAccent);
    drawCenteredText(app, app.popup.ok, "OK", kTextLight);
}

void render(App& app) {
    app.canvas.clear(pageBackground(app));
    if (app.layoutRoot) vpp::paint(*app.layoutRoot, app.canvas, app.fonts, app.scale);
    app.shell.paint(app.canvas, app.fonts, app.scale);
    if (app.popup.visible) drawPopup(app);
    if (app.pagePrefs.cornerRadius > 0 && !(SDL_GetWindowFlags(app.window) & SDL_WINDOW_MAXIMIZED))
        app.canvas.maskRoundedCorners(px(app.pagePrefs.cornerRadius, app.scale));

    SDL_UpdateTexture(app.texture, nullptr, app.canvas.pixels(), app.canvas.pitch());
    SDL_SetRenderDrawColor(app.renderer, 0, 0, 0, 0);
    SDL_RenderClear(app.renderer);
    SDL_RenderTexture(app.renderer, app.texture, nullptr, nullptr);
    SDL_RenderPresent(app.renderer);
    app.dirty = false;
}

bool recreateTexture(App& app, int w, int h) {
    if (app.texture) SDL_DestroyTexture(app.texture);
    app.texture = SDL_CreateTexture(app.renderer, SDL_PIXELFORMAT_ARGB8888, SDL_TEXTUREACCESS_STREAMING, w, h);
    if (!app.texture) {
        std::printf("SDL_CreateTexture failed: %s\n", SDL_GetError());
        return false;
    }
    SDL_SetTextureScaleMode(app.texture, SDL_SCALEMODE_NEAREST);
    SDL_SetTextureBlendMode(app.texture, SDL_BLENDMODE_BLEND); // transparent corners
    return true;
}

void updateSize(App& app) {
    int w = 0, h = 0;
    SDL_GetWindowSizeInPixels(app.window, &w, &h);
    if (w <= 0 || h <= 0) return;
    app.canvas.resize(w, h);
    recreateTexture(app, w, h);
    const float scale = SDL_GetWindowDisplayScale(app.window);
    if (scale > 0) app.scale = scale;
    layoutPopup(app);
    relayout(app);
}

// --- input ---------------------------------------------------------------------------

void toPixels(const App& app, float wx, float wy, int& x, int& y) {
    int ww = 0, wh = 0;
    SDL_GetWindowSize(app.window, &ww, &wh);
    const float ratio = ww > 0 ? static_cast<float>(app.canvas.width()) / ww : 1.0f;
    x = static_cast<int>(wx * ratio);
    y = static_cast<int>(wy * ratio);
}

const vpp::Node* elementAt(const App& app, int x, int y) {
    if (!app.layoutRoot) return nullptr;
    const vpp::LayoutBox* box = vpp::hitTest(*app.layoutRoot, x / app.scale, y / app.scale);
    return box ? box->node : nullptr;
}

void focusAddress(App& app, bool on) {
    app.shell.setFocused(on);
    if (on) {
        app.shell.selectAll = true;
        SDL_StartTextInput(app.window);
    } else {
        SDL_StopTextInput(app.window);
    }
    app.needsLayout = true;
    app.dirty = true;
}

// Ctrl+L: reveal the full shell whatever the site asked for, and edit the address.
void revealShell(App& app) {
    app.shell.force(true);
    focusAddress(app, true);
}

void onShellControl(App& app, const std::string& id) {
    if (id == "back") goBack(app);
    else if (id == "forward") goForward(app);
    else if (id == "reload") { loadPage(app); relayout(app); }
    else if (id == "minimize") SDL_MinimizeWindow(app.window);
    else if (id == "maximize") {
        if (SDL_GetWindowFlags(app.window) & SDL_WINDOW_MAXIMIZED) SDL_RestoreWindow(app.window);
        else SDL_MaximizeWindow(app.window);
    } else if (id == "close") app.running = false;
    else if (id == "address") focusAddress(app, true);
}

void onMouseDown(App& app, int x, int y) {
    if (app.popup.visible) {
        if (app.popup.ok.contains(x, y)) {
            app.popup.okPressed = true;
            app.dirty = true;
        }
        return;
    }
    if (app.shell.contains(x / app.scale, y / app.scale)) return;
    if (app.shell.focused) focusAddress(app, false);
    app.pressedNode = elementAt(app, x, y);
}

void onMouseUp(App& app, int x, int y) {
    if (app.popup.visible) {
        if (app.popup.okPressed && app.popup.ok.contains(x, y)) app.popup.visible = false;
        app.popup.okPressed = false;
        app.dirty = true;
        return;
    }
    if (app.shell.contains(x / app.scale, y / app.scale)) {
        const std::string control = app.shell.controlAt(x / app.scale, y / app.scale);
        if (control != "address" && app.shell.focused) focusAddress(app, false);
        if (!control.empty()) onShellControl(app, control);
        return;
    }

    const vpp::Node* node = elementAt(app, x, y);
    const bool click = node && node == app.pressedNode;
    app.pressedNode = nullptr;
    if (!click) return;

    // Read the link before scripts run: a handler may change the DOM.
    std::string href;
    if (const vpp::Node* a = node->closest("a"))
        if (const std::string* h = a->attribute("href")) href = *h;

    if (app.script) app.script->dispatchClick(*node);
    if (!href.empty()) {
        const std::string target = resolveLink(app, href);
        if (!target.empty()) {
            std::printf("navigate: %s\n", target.c_str());
            navigate(app, target, true);
        }
    }
}

// Frameless window behaviour: the window edges resize, the shell bar drags
// (except over its controls), and with the shell hidden the top strip of
// the page drags unless a button or link sits under the cursor.
SDL_HitTestResult SDLCALL hitTestWindow(SDL_Window* window, const SDL_Point* area, void* data) {
    const App* app = static_cast<const App*>(data);
    if (app->popup.visible) return SDL_HITTEST_NORMAL;
    int x = 0, y = 0;
    toPixels(*app, static_cast<float>(area->x), static_cast<float>(area->y), x, y);
    const float cx = x / app->scale;
    const float cy = y / app->scale;

    if (!(SDL_GetWindowFlags(window) & SDL_WINDOW_MAXIMIZED)) {
        const float w = app->canvas.width() / app->scale;
        const float h = app->canvas.height() / app->scale;
        const bool left = cx < kResizeBorder, right = cx > w - kResizeBorder;
        const bool top = cy < kResizeBorder, bottom = cy > h - kResizeBorder;
        if (top && left) return SDL_HITTEST_RESIZE_TOPLEFT;
        if (top && right) return SDL_HITTEST_RESIZE_TOPRIGHT;
        if (bottom && left) return SDL_HITTEST_RESIZE_BOTTOMLEFT;
        if (bottom && right) return SDL_HITTEST_RESIZE_BOTTOMRIGHT;
        if (top) return SDL_HITTEST_RESIZE_TOP;
        if (bottom) return SDL_HITTEST_RESIZE_BOTTOM;
        if (left) return SDL_HITTEST_RESIZE_LEFT;
        if (right) return SDL_HITTEST_RESIZE_RIGHT;
    }

    if (app->shell.contains(cx, cy))
        return app->shell.controlAt(cx, cy).empty() ? SDL_HITTEST_DRAGGABLE : SDL_HITTEST_NORMAL;

    if (app->shell.height() == 0) {
        const vpp::Node* node = elementAt(*app, x, y);
        const bool interactive = node && (node->closest("button") || node->closest("a"));
        if (cy < kDragStripHeight && !interactive) return SDL_HITTEST_DRAGGABLE;
    }
    return SDL_HITTEST_NORMAL;
}

// Typing into the address field.
void onTextInput(App& app, const std::string& text) {
    if (!app.shell.focused) return;
    if (app.shell.selectAll) {
        app.shell.address.clear();
        app.shell.selectAll = false;
    }
    app.shell.setAddress(app.shell.address + text);
    app.needsLayout = true;
}

void submitAddress(App& app) {
    std::string target = app.shell.address;
    while (!target.empty() && std::isspace(static_cast<unsigned char>(target.back()))) target.pop_back();
    while (!target.empty() && std::isspace(static_cast<unsigned char>(target.front()))) target.erase(0, 1);
    focusAddress(app, false);
    app.shell.force(false);
    if (target.empty()) return;
    // A bare host or path without a scheme is treated as https.
    if (!vpp::isRemoteUrl(target) && target.find('.') != std::string::npos && !fs::exists(fs::u8path(target)) &&
        target.find('/') != std::string::npos && target.find('\\') == std::string::npos && target[0] != '.')
        target = "https://" + target;
    navigate(app, target, true);
}

void handleEvent(App& app, const SDL_Event& e) {
    switch (e.type) {
    case SDL_EVENT_QUIT:
        app.running = false;
        break;

    case SDL_EVENT_TEXT_INPUT:
        onTextInput(app, e.text.text ? e.text.text : "");
        break;

    case SDL_EVENT_KEY_DOWN: {
        const bool alt = (e.key.mod & SDL_KMOD_ALT) != 0;
        const bool ctrl = (e.key.mod & SDL_KMOD_CTRL) != 0;

        if (app.shell.focused) {
            if (e.key.key == SDLK_ESCAPE) {
                app.shell.setAddress(app.location);
                focusAddress(app, false);
                app.shell.force(false);
            } else if (e.key.key == SDLK_RETURN || e.key.key == SDLK_KP_ENTER) {
                submitAddress(app);
            } else if (e.key.key == SDLK_BACKSPACE) {
                if (app.shell.selectAll) app.shell.address.clear();
                else if (!app.shell.address.empty()) app.shell.address.pop_back();
                app.shell.selectAll = false;
                app.shell.setAddress(app.shell.address);
                app.needsLayout = true;
            } else if (ctrl && e.key.key == SDLK_A) {
                app.shell.selectAll = true;
            } else if (ctrl && e.key.key == SDLK_V) {
                if (char* clip = SDL_GetClipboardText()) {
                    onTextInput(app, clip);
                    SDL_free(clip);
                }
            } else if (ctrl && e.key.key == SDLK_L) {
                app.shell.selectAll = true;
            }
            break;
        }

        if (e.key.key == SDLK_ESCAPE) {
            if (app.popup.visible) {
                app.popup.visible = false;
                app.dirty = true;
            } else if (app.shell.forced()) {
                app.shell.force(false);
                app.needsLayout = true;
                app.dirty = true;
            } else {
                app.running = false;
            }
        } else if (ctrl && e.key.key == SDLK_L) {
            revealShell(app);
        } else if (e.key.key == SDLK_R || e.key.key == SDLK_F5) {
            loadPage(app);
            relayout(app);
        } else if (alt && e.key.key == SDLK_LEFT) {
            goBack(app);
        } else if (alt && e.key.key == SDLK_RIGHT) {
            goForward(app);
        }
        break;
    }

    case SDL_EVENT_MOUSE_BUTTON_DOWN:
    case SDL_EVENT_MOUSE_BUTTON_UP: {
        const bool down = e.type == SDL_EVENT_MOUSE_BUTTON_DOWN;
        if (e.button.button == SDL_BUTTON_X1 && down) { goBack(app); break; }
        if (e.button.button == SDL_BUTTON_X2 && down) { goForward(app); break; }
        if (e.button.button != SDL_BUTTON_LEFT) break;
        int x = 0, y = 0;
        toPixels(app, e.button.x, e.button.y, x, y);
        if (down) onMouseDown(app, x, y);
        else onMouseUp(app, x, y);
        break;
    }

    case SDL_EVENT_WINDOW_PIXEL_SIZE_CHANGED:
    case SDL_EVENT_WINDOW_DISPLAY_SCALE_CHANGED:
        updateSize(app);
        break;

    case SDL_EVENT_WINDOW_EXPOSED:
        app.dirty = true;
        break;

    default:
        break;
    }
}

} // namespace

int main(int argc, char** argv) {
    if (!SDL_Init(SDL_INIT_VIDEO)) {
        std::printf("SDL_Init failed: %s\n", SDL_GetError());
        return 1;
    }

    App app;
    for (int i = 1; i < argc; ++i) {
        const std::string arg = argv[i];
        if (arg == "--allow-unsigned") app.allowUnsigned = true;
        else if (app.location.empty()) app.location = arg;
    }
    // An empty location is the start page.
    app.history.push_back(app.location);
    app.historyIndex = 0;

    float scale = SDL_GetDisplayContentScale(SDL_GetPrimaryDisplay());
    if (scale <= 0) scale = 1.0f;

    app.window = SDL_CreateWindow("VPP Viewer", px(kLogicalWidth, scale), px(kLogicalHeight, scale),
                                  SDL_WINDOW_BORDERLESS | SDL_WINDOW_RESIZABLE | SDL_WINDOW_HIGH_PIXEL_DENSITY |
                                      SDL_WINDOW_TRANSPARENT);
    if (!app.window) {
        std::printf("SDL_CreateWindow failed: %s\n", SDL_GetError());
        SDL_Quit();
        return 1;
    }

    app.renderer = SDL_CreateRenderer(app.window, nullptr);
    if (!app.renderer) {
        std::printf("SDL_CreateRenderer failed: %s\n", SDL_GetError());
        SDL_DestroyWindow(app.window);
        SDL_Quit();
        return 1;
    }

    loadFonts(app.fonts);
    // Window preferences applied while loading (width, height) need the real scale.
    const float windowScale = SDL_GetWindowDisplayScale(app.window);
    if (windowScale > 0) app.scale = windowScale;
    loadPage(app);
    updateSize(app);
    if (!app.texture) {
        SDL_DestroyRenderer(app.renderer);
        SDL_DestroyWindow(app.window);
        SDL_Quit();
        return 1;
    }
    SDL_SetWindowHitTest(app.window, hitTestWindow, &app);

    while (app.running) {
        SDL_Event e;
        if (!SDL_WaitEvent(&e)) break;
        handleEvent(app, e);
        while (SDL_PollEvent(&e)) handleEvent(app, e);

        if (app.needsLayout) relayout(app);
        if (app.dirty && app.running) render(app);
    }

    SDL_DestroyTexture(app.texture);
    SDL_DestroyRenderer(app.renderer);
    SDL_DestroyWindow(app.window);
    SDL_Quit();
    return 0;
}
