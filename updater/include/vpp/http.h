#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace vpp {

// Fetches a URL with GET. Supports http://, https://, and vpp:// (which maps
// to https://). Fails on any status other than 200.
bool httpGet(const std::string& url, std::vector<uint8_t>& out, std::string* error);

// True for http://, https://, and vpp:// URLs.
bool isRemoteUrl(const std::string& s);

} // namespace vpp
