#include "vpp/updater.h"

#include "vpp/package.h"
#include "vpp/sha256.h"

#include <algorithm>
#include <cctype>
#include <cstdlib>
#include <fstream>
#include <map>
#include <sstream>

namespace fs = std::filesystem;

namespace vpp {

namespace {

bool readFile(const fs::path& path, std::vector<uint8_t>& out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) return false;
    std::stringstream buffer;
    buffer << in.rdbuf();
    const std::string s = buffer.str();
    out.assign(s.begin(), s.end());
    return true;
}

// Writes to a temporary file next to the target, then renames into place.
bool writeAtomic(const fs::path& path, const std::vector<uint8_t>& bytes, std::string* error) {
    std::error_code ec;
    fs::create_directories(path.parent_path(), ec);
    const fs::path tmp = path.string() + ".tmp";
    {
        std::ofstream out(tmp, std::ios::binary | std::ios::trunc);
        if (!out) {
            if (error) *error = "cannot write " + tmp.string();
            return false;
        }
        out.write(reinterpret_cast<const char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
        if (!out) {
            if (error) *error = "cannot write " + tmp.string();
            return false;
        }
    }
    fs::rename(tmp, path, ec);
    if (ec) {
        fs::remove(path, ec);
        fs::rename(tmp, path, ec);
        if (ec) {
            if (error) *error = "cannot replace " + path.string();
            return false;
        }
    }
    return true;
}

bool endsWith(const std::string& s, const std::string& suffix) {
    return s.size() >= suffix.size() && s.compare(s.size() - suffix.size(), suffix.size(), suffix) == 0;
}

std::string baseUrl(const std::string& url) {
    const size_t slash = url.find_last_of('/');
    return slash == std::string::npos ? url + "/" : url.substr(0, slash + 1);
}

void say(const std::function<void(const std::string&)>& log, const std::string& line) {
    if (log) log(line);
}

} // namespace

std::string sanitizeId(const std::string& id) {
    std::string out;
    for (unsigned char c : id)
        out.push_back(std::isalnum(c) || c == '.' || c == '-' || c == '_' ? static_cast<char>(c) : '_');
    return out.empty() ? "app" : out;
}

int compareVersions(const std::string& a, const std::string& b) {
    size_t i = 0, j = 0;
    while (i < a.size() || j < b.size()) {
        std::string sa, sb;
        while (i < a.size() && a[i] != '.') sa.push_back(a[i++]);
        while (j < b.size() && b[j] != '.') sb.push_back(b[j++]);
        if (i < a.size()) ++i;
        if (j < b.size()) ++j;

        const bool numeric = !sa.empty() && !sb.empty() &&
                             std::all_of(sa.begin(), sa.end(), [](unsigned char c) { return std::isdigit(c); }) &&
                             std::all_of(sb.begin(), sb.end(), [](unsigned char c) { return std::isdigit(c); });
        if (numeric) {
            const long long na = std::strtoll(sa.c_str(), nullptr, 10);
            const long long nb = std::strtoll(sb.c_str(), nullptr, 10);
            if (na != nb) return na < nb ? -1 : 1;
        } else if (sa != sb) {
            if (sa.empty()) return -1;
            if (sb.empty()) return 1;
            return sa < sb ? -1 : 1;
        }
    }
    return 0;
}

AppStore::AppStore(fs::path root) : root_(std::move(root)) {}

bool AppStore::sync(const std::string& url, const Fetch& fetch, SyncResult& result, std::string* error,
                    const std::function<void(const std::string&)>& log) {
    result = SyncResult{};

    // The page URL may name the package or its manifest; derive both.
    std::string manifestUrl = url;
    std::string packageUrl = url;
    if (endsWith(url, ".vpp")) manifestUrl = url.substr(0, url.size() - 4) + ".vppm";
    else if (endsWith(url, ".vppm")) packageUrl = url.substr(0, url.size() - 5) + ".vpp";

    // 1. Fetch the manifest; fall back to the whole package; then to the cache.
    std::vector<uint8_t> remoteBytes;
    std::string fetchError;
    Package remote;
    Package full;
    bool haveFull = false;

    if (fetch(manifestUrl, remoteBytes, &fetchError)) {
        if (!Package::openHeader(remoteBytes, remote, error)) return false;
    } else {
        std::vector<uint8_t> fullBytes;
        std::string secondError;
        if (fetch(packageUrl, fullBytes, &secondError)) {
            if (!Package::open(std::move(fullBytes), full, error)) return false;
            haveFull = true;
            remoteBytes = full.headerBytes();
            if (!Package::openHeader(remoteBytes, remote, error)) return false;
            say(log, "no manifest served; downloaded the whole page package");
        } else {
            // Offline: look for an installed page that came from this URL.
            std::error_code ec;
            for (const auto& site : fs::directory_iterator(root_, ec)) {
                for (const auto& f : fs::directory_iterator(site.path() / "pages", ec)) {
                    if (f.path().extension() != ".url") continue;
                    std::vector<uint8_t> src;
                    if (!readFile(f.path(), src)) continue;
                    const std::string from(src.begin(), src.end());
                    if (from != url && from != manifestUrl && from != packageUrl) continue;
                    const fs::path pkgPath = f.path().parent_path() / (f.path().stem().string() + ".vpp");
                    if (!fs::exists(pkgPath)) continue;
                    std::vector<uint8_t> hdr;
                    Package cached;
                    if (readFile(f.path().parent_path() / (f.path().stem().string() + ".vppm"), hdr) &&
                        Package::openHeader(hdr, cached, nullptr)) {
                        result.siteId = cached.manifest().id;
                        result.page = cached.manifest().page;
                        result.version = cached.manifest().version;
                    }
                    result.packagePath = pkgPath;
                    result.offline = true;
                    say(log, "offline: " + fetchError + "; running the installed copy");
                    return true;
                }
            }
            if (error) *error = fetchError;
            return false;
        }
    }

    // 2. Only signed pages are ever installed from the network.
    if (remote.verifySignature() != SignatureStatus::Valid) {
        if (error) *error = remote.isSigned() ? "remote manifest signature is invalid" : "remote manifest is unsigned";
        return false;
    }

    const AppManifest& m = remote.manifest();
    const std::string page = m.page.empty() ? "index" : sanitizeId(m.page);
    const fs::path siteDir = root_ / sanitizeId(m.id);
    const fs::path pagesDir = siteDir / "pages";
    const fs::path resDir = siteDir / "res";
    std::error_code ec;
    fs::create_directories(pagesDir, ec);
    fs::create_directories(resDir, ec);
    result.siteId = m.id;
    result.page = m.page;
    result.version = m.version;
    result.packagePath = pagesDir / (page + ".vpp");
    const fs::path manifestPath = pagesDir / (page + ".vppm");

    // 3. Compare with the installed copy of this page: same publisher, no rollback.
    std::vector<uint8_t> installedBytes;
    Package installed;
    const bool haveInstalled = readFile(manifestPath, installedBytes) &&
                               Package::openHeader(installedBytes, installed, nullptr);
    if (haveInstalled) {
        if (installed.publisherKey() != remote.publisherKey()) {
            if (error) *error = "publisher key changed for " + m.id + "; refusing the update";
            return false;
        }
        const int cmp = compareVersions(m.version, installed.manifest().version);
        if (cmp < 0) {
            if (error) *error = "rollback refused: server offers " + m.version + " but " +
                                installed.manifest().version + " is installed";
            return false;
        }
        if (installedBytes == remoteBytes && fs::exists(result.packagePath)) {
            say(log, "up to date: " + m.name + " / " + page + " " + m.version);
            return true;
        }
    }

    // 4. Gather resources: from the store, the downloaded package, or the server.
    const std::string base = baseUrl(url);
    std::map<std::string, std::vector<uint8_t>> resources;
    for (const PackageEntry& e : remote.entries()) {
        const std::string hex = toHex(e.sha256);
        const fs::path local = resDir / hex;
        std::vector<uint8_t> bytes;
        if (readFile(local, bytes) && bytes.size() == e.size && sha256(bytes) == e.sha256) {
            say(log, "cached: " + e.name + " (" + std::to_string(bytes.size()) + " bytes)");
            resources[e.name] = std::move(bytes);
            continue;
        }
        if (haveFull) {
            if (!full.read(e.name, bytes, error)) return false;
        } else {
            if (!fetch(base + "res/" + hex, bytes, error)) return false;
            if (bytes.size() != e.size || sha256(bytes) != e.sha256) {
                if (error) *error = "downloaded " + e.name + " does not match its hash";
                return false;
            }
        }
        if (!writeAtomic(local, bytes, error)) return false;
        say(log, "downloaded: " + e.name + " (" + std::to_string(bytes.size()) + " bytes)");
        ++result.downloaded;
        result.downloadedBytes += bytes.size();
        resources[e.name] = std::move(bytes);
    }

    // 5. Assemble and install. The manifest goes last.
    std::vector<uint8_t> assembled;
    if (!Package::assemble(remote, resources, assembled, error)) return false;
    if (!writeAtomic(result.packagePath, assembled, error)) return false;
    if (!writeAtomic(manifestPath, remoteBytes, error)) return false;
    writeAtomic(pagesDir / (page + ".url"), std::vector<uint8_t>(url.begin(), url.end()), nullptr);

    result.updated = true;
    say(log, std::string(haveInstalled ? "updated: " : "installed: ") + m.name + " / " + page + " " + m.version +
                 " (" + std::to_string(result.downloaded) + " resources, " + std::to_string(result.downloadedBytes) +
                 " bytes downloaded)");
    return true;
}

} // namespace vpp
