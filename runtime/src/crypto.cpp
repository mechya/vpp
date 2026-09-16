// Ed25519 signatures via the orlp/ed25519 library. Nothing outside this file
// includes it.

#include "vpp/crypto.h"

#include <ed25519.h>

#include <cctype>
#include <fstream>
#include <sstream>

namespace vpp {

namespace {

std::string trim(const std::string& s) {
    size_t a = 0;
    size_t b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

// First non-empty line that is not a '#' comment.
bool readKeyLine(const std::string& path, std::string& out) {
    std::ifstream in(path);
    if (!in) return false;
    std::string line;
    while (std::getline(in, line)) {
        line = trim(line);
        if (line.empty() || line[0] == '#') continue;
        out = line;
        return true;
    }
    return false;
}

} // namespace

SigningKey signingKeyFromSeed(const SecretSeed& seed) {
    SigningKey key;
    key.seed = seed;
    ed25519_create_keypair(key.publicKey.data(), key.privateKey.data(), seed.data());
    return key;
}

bool generateSigningKey(SigningKey& out) {
    SecretSeed seed{};
    if (ed25519_create_seed(seed.data()) != 0) return false;
    out = signingKeyFromSeed(seed);
    return true;
}

Signature sign(const SigningKey& key, const uint8_t* message, size_t size) {
    Signature sig{};
    ed25519_sign(sig.data(), message, size, key.publicKey.data(), key.privateKey.data());
    return sig;
}

bool verify(const PublicKey& key, const Signature& signature, const uint8_t* message, size_t size) {
    return ed25519_verify(signature.data(), message, size, key.data()) == 1;
}

std::string hexEncode(const uint8_t* data, size_t size) {
    static const char* digits = "0123456789abcdef";
    std::string out;
    out.reserve(size * 2);
    for (size_t i = 0; i < size; ++i) {
        out.push_back(digits[data[i] >> 4]);
        out.push_back(digits[data[i] & 15]);
    }
    return out;
}

bool hexDecode(const std::string& hex, uint8_t* out, size_t size) {
    if (hex.size() != size * 2) return false;
    auto nibble = [](char c) -> int {
        if (c >= '0' && c <= '9') return c - '0';
        if (c >= 'a' && c <= 'f') return c - 'a' + 10;
        if (c >= 'A' && c <= 'F') return c - 'A' + 10;
        return -1;
    };
    for (size_t i = 0; i < size; ++i) {
        const int hi = nibble(hex[i * 2]);
        const int lo = nibble(hex[i * 2 + 1]);
        if (hi < 0 || lo < 0) return false;
        out[i] = static_cast<uint8_t>((hi << 4) | lo);
    }
    return true;
}

bool saveSigningKey(const SigningKey& key, const std::string& secretPath, const std::string& publicPath,
                    std::string* error) {
    std::ofstream secret(secretPath, std::ios::trunc);
    if (!secret) {
        if (error) *error = "cannot write " + secretPath;
        return false;
    }
    secret << "# VPP publisher secret key (Ed25519 seed). Keep this file private.\n"
           << hexEncode(key.seed.data(), key.seed.size()) << "\n";

    std::ofstream pub(publicPath, std::ios::trunc);
    if (!pub) {
        if (error) *error = "cannot write " + publicPath;
        return false;
    }
    pub << "# VPP publisher public key (Ed25519).\n" << hexEncode(key.publicKey.data(), key.publicKey.size()) << "\n";
    return true;
}

bool loadSigningKey(const std::string& secretPath, SigningKey& out, std::string* error) {
    std::string hex;
    if (!readKeyLine(secretPath, hex)) {
        if (error) *error = "cannot read key file " + secretPath;
        return false;
    }
    SecretSeed seed{};
    if (!hexDecode(hex, seed.data(), seed.size())) {
        if (error) *error = secretPath + " does not contain a 64-character hex seed";
        return false;
    }
    out = signingKeyFromSeed(seed);
    return true;
}

} // namespace vpp
