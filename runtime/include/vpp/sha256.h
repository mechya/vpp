#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace vpp {

using Sha256Digest = std::array<uint8_t, 32>;

Sha256Digest sha256(const uint8_t* data, size_t size);
Sha256Digest sha256(const std::vector<uint8_t>& data);

// Lower-case hex, 64 characters.
std::string toHex(const Sha256Digest& digest);

} // namespace vpp
