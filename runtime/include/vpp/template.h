#pragma once

#include "vpp/css.h"
#include "vpp/dom.h"

#include <filesystem>
#include <map>
#include <memory>
#include <string>
#include <vector>

namespace vpp {

// Build-time templates: includes, layouts with slots, and components with
// literal properties and scoped CSS. See docs/template-syntax.md. Everything
// here resolves to a plain document; nothing template-related survives into
// a package.

struct TemplateOptions {
    std::filesystem::path projectRoot;                   // "/x" references resolve here
    std::map<std::string, std::string> componentAliases; // tag name -> project path of the component directory
};

// A stylesheet the page uses, already parsed (and scoped, for components).
struct PageStyle {
    std::string name;            // resource name, e.g. "global" or "component-stat-card"
    std::filesystem::path path;  // empty for inline <style>
    StyleSheet sheet;
};

struct PageScript {
    std::string name;            // resource name, e.g. "app"
    std::filesystem::path path;  // empty for inline <script>
    std::string source;
};

struct ExpandedPage {
    std::unique_ptr<Node> document;
    std::vector<PageStyle> styles;   // in load order
    std::vector<PageScript> scripts; // in load order
    std::vector<std::string> warnings;
};

// Text-level fixes applied before HTML parsing: self-closing custom tags
// become open/close pairs, and vpp-slot elements inside <head> become
// <meta name="vpp-slot"> placeholders so the parser leaves them in place.
std::string preprocessTemplateHtml(const std::string& html);

// Loads a page, applies its layout, expands includes and components, and
// collects its stylesheets and scripts in document order. Works for plain
// pages without any template elements too.
bool expandPage(const std::filesystem::path& pageFile, const TemplateOptions& options, ExpandedPage& out,
                std::string* error);

// The directory containing vpp.json, searching upward from file; the file's
// own directory if none is found.
std::filesystem::path findProjectRoot(const std::filesystem::path& file);

// Reads component aliases ("components": {...}) from a vpp.json text.
std::map<std::string, std::string> parseComponentAliases(const std::string& json);

} // namespace vpp
