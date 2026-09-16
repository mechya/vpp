#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>

namespace vpp {

// Publisher identity: Ed25519 keys. The public key travels inside packages;
// the secret seed stays on the publisher's machine.

using PublicKey = std::array<uint8_t, 32>;
using SecretSeed = std::array<uint8_t, 32>;
using Signature = std::array<uint8_t, 64>;

struct SigningKey {
    SecretSeed seed{};
    PublicKey publicKey{};
    std::array<uint8_t, 64> privateKey{}; // expanded form used for signing, derived from seed
};

// Random seed from the operating system. False if no entropy source is available.
bool generateSigningKey(SigningKey& out);
SigningKey signingKeyFromSeed(const SecretSeed& seed);

Signature sign(const SigningKey& key, const uint8_t* message, size_t size);
bool verify(const PublicKey& key, const Signature& signature, const uint8_t* message, size_t size);

std::string hexEncode(const uint8_t* data, size_t size);
bool hexDecode(const std::string& hex, uint8_t* out, size_t size);

// Key files: a comment line starting with '#', then the key in hex.
bool saveSigningKey(const SigningKey& key, const std::string& secretPath, const std::string& publicPath,
                    std::string* error);
bool loadSigningKey(const std::string& secretPath, SigningKey& out, std::string* error);

} // namespace vpp
