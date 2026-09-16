#pragma once

#include "vpp/dom.h"

#include <memory>
#include <string>
#include <vector>

namespace vpp {

// Parses HTML source into a VPP document. Never returns null; a document with
// no children means the input could not be parsed.
std::unique_ptr<Node> parseHtml(const std::string& source);

// Reads and parses a file. Returns null and sets error if the file cannot be read.
std::unique_ptr<Node> parseHtmlFile(const std::string& path, std::string* error);

// A stylesheet or script referenced by a page: either a relative file path
// (isFile) or inline text.
struct PageResource {
    bool isFile = false;
    std::string value;
};

struct PageResources {
    std::vector<PageResource> styles;  // <link rel="stylesheet" href> and <style>, in order
    std::vector<PageResource> scripts; // <script src> and inline <script>, in order
};

PageResources collectPageResources(const Node& document);

} // namespace vpp
