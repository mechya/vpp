#pragma once

#include <cstdint>
#include <filesystem>
#include <functional>
#include <string>
#include <vector>

namespace vpp {

// Hosting layout, produced by `vpppack --publish`, served by any static file
// server:
//
//   <site>/<page>.vpp          full page package
//   <site>/<page>.vppm         its signed header, for update checks
//   <site>/res/<sha256 hex>    resources, shared by every page of the site
//
// A page URL names the .vpp; the viewer checks the .vppm first and falls
// back to downloading the whole .vpp when no manifest is served.

using Fetch = std::function<bool(const std::string& url, std::vector<uint8_t>& out, std::string* error)>;

struct SyncResult {
    std::filesystem::path packagePath; // the assembled .vpp to run
    std::string siteId;
    std::string page;
    std::string version;
    bool offline = false;              // the server was unreachable; running the installed copy
    bool updated = false;              // a new manifest was installed
    int downloaded = 0;                // resources fetched
    uint64_t downloadedBytes = 0;
};

// The local store of installed pages:
//
//   <root>/<site id>/pages/<page>.vppm   the installed header
//   <root>/<site id>/pages/<page>.vpp    assembled package
//   <root>/<site id>/pages/<page>.url    where it came from
//   <root>/<site id>/res/<sha256>        resources, shared across pages and versions
//
// Files are written to temporary names and renamed into place, and a page's
// manifest is replaced last, so an interrupted update never damages the
// installed version.
class AppStore {
public:
    explicit AppStore(std::filesystem::path root);

    // Fetches the page's manifest (or the whole package), verifies its
    // signature, refuses rollbacks and publisher changes against the
    // installed copy, downloads only the resources not already present, and
    // installs. If the server cannot be reached and a cached copy exists,
    // returns that copy with offline set.
    bool sync(const std::string& url, const Fetch& fetch, SyncResult& result, std::string* error,
              const std::function<void(const std::string&)>& log = nullptr);

    const std::filesystem::path& root() const { return root_; }

private:
    std::filesystem::path root_;
};

// Compares dotted version strings segment by segment: numeric where both
// segments are numbers, lexical otherwise. Returns <0, 0, >0.
int compareVersions(const std::string& a, const std::string& b);

// Makes a site or page id safe to use as a file name.
std::string sanitizeId(const std::string& id);

} // namespace vpp
