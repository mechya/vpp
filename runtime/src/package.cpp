#include "vpp/package.h"

#include "vpp/bytes.h"

#include <algorithm>
#include <cctype>
#include <set>

namespace vpp {

namespace {

constexpr uint16_t kPackageVersion = 3; // 2: page name; 3: window preferences
const char kMagic[4] = {'V', 'P', 'P', 'K'};

// --- minimal JSON: one flat object of scalar values -----------------------------

class JsonReader {
public:
    explicit JsonReader(const std::string& text) : s_(text) {}

    bool parseObject(std::map<std::string, std::string>& out, std::string* error) {
        skipSpace();
        if (!expect('{')) return fail(error, "expected '{'");
        skipSpace();
        if (peek() == '}') {
            ++i_;
            return true;
        }
        while (true) {
            skipSpace();
            std::string key;
            if (!parseString(key)) return fail(error, "expected a quoted key");
            skipSpace();
            if (!expect(':')) return fail(error, "expected ':' after \"" + key + "\"");
            skipSpace();
            std::string value;
            if (!parseScalar(value)) return fail(error, "unsupported value for \"" + key + "\"");
            out[key] = value;
            skipSpace();
            if (peek() == ',') {
                ++i_;
                continue;
            }
            if (expect('}')) return true;
            return fail(error, "expected ',' or '}'");
        }
    }

private:
    char peek() const { return i_ < s_.size() ? s_[i_] : '\0'; }
    bool expect(char c) {
        if (peek() != c) return false;
        ++i_;
        return true;
    }
    void skipSpace() {
        while (i_ < s_.size() && std::isspace(static_cast<unsigned char>(s_[i_]))) ++i_;
    }
    static bool fail(std::string* error, const std::string& why) {
        if (error) *error = "vpp.json: " + why;
        return false;
    }

    static void appendUtf8(std::string& out, unsigned code) {
        if (code < 0x80) {
            out.push_back(static_cast<char>(code));
        } else if (code < 0x800) {
            out.push_back(static_cast<char>(0xC0 | (code >> 6)));
            out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
        } else {
            out.push_back(static_cast<char>(0xE0 | (code >> 12)));
            out.push_back(static_cast<char>(0x80 | ((code >> 6) & 0x3F)));
            out.push_back(static_cast<char>(0x80 | (code & 0x3F)));
        }
    }

    bool parseString(std::string& out) {
        if (!expect('"')) return false;
        out.clear();
        while (i_ < s_.size()) {
            const char c = s_[i_++];
            if (c == '"') return true;
            if (c != '\\') {
                out.push_back(c);
                continue;
            }
            if (i_ >= s_.size()) return false;
            const char e = s_[i_++];
            switch (e) {
            case '"': out.push_back('"'); break;
            case '\\': out.push_back('\\'); break;
            case '/': out.push_back('/'); break;
            case 'b': out.push_back('\b'); break;
            case 'f': out.push_back('\f'); break;
            case 'n': out.push_back('\n'); break;
            case 'r': out.push_back('\r'); break;
            case 't': out.push_back('\t'); break;
            case 'u': {
                if (i_ + 4 > s_.size()) return false;
                unsigned code = 0;
                for (int k = 0; k < 4; ++k) {
                    const char h = s_[i_++];
                    code <<= 4;
                    if (h >= '0' && h <= '9') code |= static_cast<unsigned>(h - '0');
                    else if (h >= 'a' && h <= 'f') code |= static_cast<unsigned>(h - 'a' + 10);
                    else if (h >= 'A' && h <= 'F') code |= static_cast<unsigned>(h - 'A' + 10);
                    else return false;
                }
                appendUtf8(out, code);
                break;
            }
            default: return false;
            }
        }
        return false;
    }

    bool parseScalar(std::string& out) {
        if (peek() == '"') return parseString(out);
        if (peek() == '{' || peek() == '[') {
            // Nested objects and arrays are skipped: the manifest reader only
            // uses the top-level strings. (Component aliases are read elsewhere.)
            const size_t start = i_;
            int depth = 0;
            char quote = 0;
            while (i_ < s_.size()) {
                const char c = s_[i_++];
                if (quote) {
                    if (c == '\\') ++i_;
                    else if (c == quote) quote = 0;
                } else if (c == '"') {
                    quote = c;
                } else if (c == '{' || c == '[') {
                    ++depth;
                } else if (c == '}' || c == ']') {
                    if (--depth == 0) break;
                }
            }
            out = s_.substr(start, i_ - start);
            return depth == 0;
        }
        const size_t start = i_;
        while (i_ < s_.size() && (std::isalnum(static_cast<unsigned char>(s_[i_])) || s_[i_] == '.' ||
                                  s_[i_] == '-' || s_[i_] == '+'))
            ++i_;
        if (i_ == start) return false;
        out = s_.substr(start, i_ - start);
        return true;
    }

    const std::string& s_;
    size_t i_ = 0;
};

} // namespace

bool parseAppManifest(const std::string& json, AppManifest& out, std::string* error) {
    std::map<std::string, std::string> fields;
    JsonReader reader(json);
    if (!reader.parseObject(fields, error)) return false;

    AppManifest m;
    m.id = fields["id"];
    m.name = fields["name"];
    m.version = fields["version"];
    m.window = fields["window"];
    if (m.id.empty()) {
        if (error) *error = "vpp.json: \"id\" is required, e.g. \"com.example.hello\"";
        return false;
    }
    if (m.name.empty()) m.name = m.id;
    if (m.version.empty()) m.version = "0.0.0";
    out = std::move(m);
    return true;
}

// --- container ------------------------------------------------------------------

std::vector<uint8_t> Package::build(const AppManifest& manifest,
                                    const std::map<std::string, std::vector<uint8_t>>& resources,
                                    const SigningKey* key) {
    ByteWriter w;
    w.magic(kMagic);
    w.u16(kPackageVersion);
    w.str(manifest.id);
    w.str(manifest.name);
    w.str(manifest.version);
    w.str(manifest.page);
    w.str(manifest.window);

    w.u32(static_cast<uint32_t>(resources.size()));
    uint64_t offset = 0;
    for (const auto& [name, bytes] : resources) {
        w.str(name);
        w.u64(offset);
        w.u64(bytes.size());
        const Sha256Digest digest = sha256(bytes);
        w.bytes(digest.data(), digest.size());
        offset += bytes.size();
    }

    // Everything written so far is the signed region.
    if (key) {
        const Signature sig = sign(*key, w.view().data(), w.size());
        w.str(std::string(key->publicKey.begin(), key->publicKey.end()));
        w.str(std::string(sig.begin(), sig.end()));
    } else {
        w.str(std::string()); // no publisher key
        w.str(std::string()); // no signature
    }

    for (const auto& [name, bytes] : resources) w.bytes(bytes);
    return w.take();
}

bool Package::open(std::vector<uint8_t> file, Package& out, std::string* error) {
    return parse(std::move(file), false, out, error);
}

bool Package::openHeader(std::vector<uint8_t> header, Package& out, std::string* error) {
    return parse(std::move(header), true, out, error);
}

std::vector<uint8_t> Package::headerBytes() const {
    return std::vector<uint8_t>(file_.begin(), file_.begin() + static_cast<std::ptrdiff_t>(dataStart_));
}

bool Package::assemble(const Package& header, const std::map<std::string, std::vector<uint8_t>>& resources,
                       std::vector<uint8_t>& out, std::string* error) {
    uint64_t total = 0;
    for (const PackageEntry& e : header.entries_) total = std::max(total, e.offset + e.size);

    std::vector<uint8_t> file = header.headerBytes();
    const size_t dataStart = file.size();
    file.resize(dataStart + static_cast<size_t>(total));

    for (const PackageEntry& e : header.entries_) {
        auto it = resources.find(e.name);
        if (it == resources.end()) {
            if (error) *error = "missing resource " + e.name;
            return false;
        }
        if (it->second.size() != e.size) {
            if (error) *error = "resource " + e.name + " has the wrong size";
            return false;
        }
        std::copy(it->second.begin(), it->second.end(), file.begin() + static_cast<std::ptrdiff_t>(dataStart + e.offset));
    }
    out = std::move(file);
    return true;
}

bool Package::parse(std::vector<uint8_t> file, bool headerOnly, Package& out, std::string* error) {
    ByteReader r(file);
    Package pkg;
    pkg.headerOnly_ = headerOnly;

    if (!r.magic(kMagic)) {
        if (error) *error = "not a .vpp package";
        return false;
    }
    if (r.u16() != kPackageVersion) {
        if (error) *error = "unsupported package version";
        return false;
    }
    pkg.manifest_.id = r.str();
    pkg.manifest_.name = r.str();
    pkg.manifest_.version = r.str();
    pkg.manifest_.page = r.str();
    pkg.manifest_.window = r.str();

    const uint32_t count = r.count();
    std::set<std::string> names;
    for (uint32_t i = 0; i < count && r.ok(); ++i) {
        PackageEntry e;
        e.name = r.str();
        e.offset = r.u64();
        e.size = r.u64();
        r.bytes(e.sha256.data(), e.sha256.size());
        if (!r.ok()) break;
        if (e.name.empty() || !names.insert(e.name).second) {
            if (error) *error = "corrupt package: bad resource name";
            return false;
        }
        pkg.entries_.push_back(std::move(e));
    }
    pkg.signedEnd_ = r.position();

    const std::string key = r.str();
    const std::string sig = r.str();
    if (!r.ok()) {
        if (error) *error = "corrupt package: " + r.error();
        return false;
    }
    if (key.size() == pkg.publisherKey_.size() && sig.size() == pkg.signature_.size()) {
        std::copy(key.begin(), key.end(), pkg.publisherKey_.begin());
        std::copy(sig.begin(), sig.end(), pkg.signature_.begin());
        pkg.signed_ = true;
    } else if (!key.empty() || !sig.empty()) {
        if (error) *error = "corrupt package: malformed signature";
        return false;
    }

    pkg.dataStart_ = r.position();
    if (headerOnly) {
        if (!r.atEnd()) {
            if (error) *error = "corrupt manifest: trailing data";
            return false;
        }
    } else {
        const uint64_t dataSize = file.size() - pkg.dataStart_;
        for (const PackageEntry& e : pkg.entries_) {
            if (e.offset > dataSize || e.size > dataSize - e.offset) {
                if (error) *error = "corrupt package: resource " + e.name + " is out of bounds";
                return false;
            }
        }
    }

    pkg.file_ = std::move(file);
    out = std::move(pkg);
    return true;
}

SignatureStatus Package::verifySignature() const {
    if (!signed_) return SignatureStatus::Unsigned;
    return verify(publisherKey_, signature_, file_.data(), signedEnd_) ? SignatureStatus::Valid
                                                                        : SignatureStatus::Invalid;
}

const PackageEntry* Package::find(const std::string& name) const {
    for (const PackageEntry& e : entries_)
        if (e.name == name) return &e;
    return nullptr;
}

bool Package::read(const std::string& name, std::vector<uint8_t>& out, std::string* error) const {
    if (headerOnly_) {
        if (error) *error = "manifest has no resource data";
        return false;
    }
    const PackageEntry* e = find(name);
    if (!e) {
        if (error) *error = "package has no " + name;
        return false;
    }
    const uint8_t* begin = file_.data() + dataStart_ + e->offset;
    out.assign(begin, begin + e->size);
    if (sha256(out) != e->sha256) {
        out.clear();
        if (error) *error = name + " failed hash verification";
        return false;
    }
    return true;
}

} // namespace vpp
