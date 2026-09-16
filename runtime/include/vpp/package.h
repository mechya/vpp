#pragma once

#include "vpp/crypto.h"
#include "vpp/sha256.h"

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace vpp {

// The application manifest a developer writes: vpp.json in the project root.
//
//   { "id": "com.example.hello", "name": "Hello World", "version": "1.0.0" }
struct AppManifest {
    std::string id;      // site identifier, reverse-DNS style, stable across versions and shared by all pages
    std::string name;    // shown to users
    std::string version; // free-form, e.g. "1.0.0"
    std::string page;    // page name within the site, e.g. "home"; set by the packager
};

// Parses the flat JSON object of vpp.json. Only string values are used.
bool parseAppManifest(const std::string& json, AppManifest& out, std::string* error);

// One resource inside a .vpp file.
struct PackageEntry {
    std::string name; // "dom.bin", "style.bin", "code.bin", ...
    uint64_t offset = 0;
    uint64_t size = 0;
    Sha256Digest sha256{};
};

enum class SignatureStatus { Unsigned, Valid, Invalid };

// A .vpp file in memory.
//
//   VPPK, version
//   manifest: id, name, version          \  signed region
//   entries: name, offset, size, sha256  /
//   publisher key (32 bytes) and signature (64 bytes), or both empty
//   resource data
//
// The signature covers the manifest and the entry table. Since the table
// holds every resource's hash, it transitively covers the data too.
class Package {
public:
    // Parses the container. Validates structure only; hashes are checked by
    // read() and the signature by verifySignature().
    static bool open(std::vector<uint8_t> file, Package& out, std::string* error);

    // Parses a header-only file (manifest.vppm): everything before the
    // resource data. Used for update checks. read() is unavailable.
    static bool openHeader(std::vector<uint8_t> header, Package& out, std::string* error);

    // Builds a package from resources, hashing each one. Signs it if a key is given.
    static std::vector<uint8_t> build(const AppManifest& manifest,
                                      const std::map<std::string, std::vector<uint8_t>>& resources,
                                      const SigningKey* key = nullptr);

    // Rebuilds a full package file from a header and the resources it lists.
    // Fails if a resource is missing or its size does not match the entry.
    static bool assemble(const Package& header, const std::map<std::string, std::vector<uint8_t>>& resources,
                         std::vector<uint8_t>& out, std::string* error);

    // The signed header: bytes before the resource data. Equal for a full
    // package and its manifest.vppm.
    std::vector<uint8_t> headerBytes() const;
    bool isHeaderOnly() const { return headerOnly_; }

    const AppManifest& manifest() const { return manifest_; }
    const std::vector<PackageEntry>& entries() const { return entries_; }
    const PackageEntry* find(const std::string& name) const;

    bool isSigned() const { return signed_; }
    const PublicKey& publisherKey() const { return publisherKey_; }
    SignatureStatus verifySignature() const;

    // Copies a resource out after verifying its hash. Fails on mismatch.
    bool read(const std::string& name, std::vector<uint8_t>& out, std::string* error) const;

private:
    static bool parse(std::vector<uint8_t> file, bool headerOnly, Package& out, std::string* error);

    AppManifest manifest_;
    std::vector<PackageEntry> entries_;
    std::vector<uint8_t> file_;
    size_t signedEnd_ = 0;  // bytes [0, signedEnd_) are what the signature covers
    size_t dataStart_ = 0;
    bool headerOnly_ = false;
    bool signed_ = false;
    PublicKey publisherKey_{};
    Signature signature_{};
};

} // namespace vpp
