#pragma once

#include "vpp/css.h"
#include "vpp/dom.h"

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

namespace vpp {

// VPP binary resources. The compiler writes them; the viewer loads them
// without an HTML or CSS parser. Decoders validate every length and count so
// a corrupt or hostile file fails cleanly instead of crashing.

// dom.bin: the document tree. <script>, <style>, and <link> elements are
// dropped, since their content lives in code.bin and style.bin, and runs of
// whitespace in text are collapsed to a single space.
std::vector<uint8_t> encodeDom(const Node& document);
std::unique_ptr<Node> decodeDom(const std::vector<uint8_t>& bytes, std::string* error);

// style.bin: parsed rules, ready for selector matching.
std::vector<uint8_t> encodeStyleSheet(const StyleSheet& sheet);
bool decodeStyleSheet(const std::vector<uint8_t>& bytes, StyleSheet& out, std::string* error);

} // namespace vpp
