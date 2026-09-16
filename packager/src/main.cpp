// vpppack: builds, signs, and inspects .vpp page packages.
//
//   vpppack keygen [-o <name>]                       create <name>.key (secret) and <name>.pub
//   vpppack <project dir> [--key <file.key>] [-o <out dir>] [--publish]
//   vpppack --inspect <file.vpp>
//
// Every page under dist/<page>/ becomes <out>/<page>.vpp, signed with the
// same key and carrying the site id from vpp.json. --publish also writes the
// hosting layout next to them: <page>.vppm manifests and a shared res/
// directory with every resource by SHA-256, so any static server can serve
// the site and the viewer downloads each resource once.

#include "vpp/crypto.h"
#include "vpp/package.h"

#include <algorithm>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <map>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace {

int usage() {
    std::printf("usage: vpppack keygen [-o <name>]\n"
                "       vpppack <project dir> [--key <file.key>] [-o <out dir>] [--publish]\n"
                "       vpppack --inspect <file.vpp>\n");
    return 2;
}

bool readFile(const fs::path& path, std::vector<uint8_t>& out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) return false;
    std::stringstream buffer;
    buffer << in.rdbuf();
    const std::string s = buffer.str();
    out.assign(s.begin(), s.end());
    return true;
}

bool writeFile(const fs::path& path, const std::vector<uint8_t>& bytes) {
    std::error_code ec;
    fs::create_directories(path.parent_path(), ec);
    std::ofstream out(path, std::ios::binary);
    if (!out) return false;
    out.write(reinterpret_cast<const char*>(bytes.data()), static_cast<std::streamsize>(bytes.size()));
    return static_cast<bool>(out);
}

std::string keyHex(const vpp::PublicKey& key) {
    return vpp::hexEncode(key.data(), key.size());
}

int keygen(std::string name) {
    if (name.empty()) name = "publisher";
    const std::string secretPath = name + ".key";
    const std::string publicPath = name + ".pub";
    if (fs::exists(secretPath)) {
        std::printf("vpppack: %s already exists; refusing to overwrite a secret key\n", secretPath.c_str());
        return 1;
    }
    vpp::SigningKey key;
    if (!vpp::generateSigningKey(key)) {
        std::printf("vpppack: no random source available\n");
        return 1;
    }
    std::string error;
    if (!vpp::saveSigningKey(key, secretPath, publicPath, &error)) {
        std::printf("vpppack: %s\n", error.c_str());
        return 1;
    }
    std::printf("vpppack: new publisher key\n  secret  %s   (keep private, never commit)\n  public  %s\n  key id  %s\n",
                secretPath.c_str(), publicPath.c_str(), keyHex(key.publicKey).c_str());
    return 0;
}

int inspect(const std::string& file) {
    std::vector<uint8_t> bytes;
    if (!readFile(file, bytes)) {
        std::printf("vpppack: cannot read %s\n", file.c_str());
        return 1;
    }
    vpp::Package pkg;
    std::string error;
    const bool full = vpp::Package::open(bytes, pkg, &error);
    if (!full && !vpp::Package::openHeader(bytes, pkg, &error)) {
        std::printf("vpppack: %s: %s\n", file.c_str(), error.c_str());
        return 1;
    }

    const vpp::AppManifest& m = pkg.manifest();
    std::printf("%s  %s\n", full ? "package " : "manifest", file.c_str());
    std::printf("site     %s\npage     %s\nname     %s\nversion  %s\n", m.id.c_str(), m.page.c_str(), m.name.c_str(),
                m.version.c_str());

    int failures = 0;
    switch (pkg.verifySignature()) {
    case vpp::SignatureStatus::Unsigned: std::printf("signed   no\n"); break;
    case vpp::SignatureStatus::Valid:
        std::printf("signed   yes, valid\npublisher %s\n", keyHex(pkg.publisherKey()).c_str());
        break;
    case vpp::SignatureStatus::Invalid:
        std::printf("signed   yes, INVALID (manifest or hashes changed after signing)\npublisher %s\n",
                    keyHex(pkg.publisherKey()).c_str());
        ++failures;
        break;
    }

    std::printf("resources\n");
    for (const vpp::PackageEntry& e : pkg.entries()) {
        std::string status = "listed";
        if (full) {
            std::vector<uint8_t> data;
            status = pkg.read(e.name, data, &error) ? "verified" : "FAILED";
            if (status == "FAILED") ++failures;
        }
        std::printf("  %-28s %8llu bytes  %s  %s\n", e.name.c_str(), static_cast<unsigned long long>(e.size),
                    vpp::toHex(e.sha256).substr(0, 16).c_str(), status.c_str());
    }
    if (failures) {
        std::printf("vpppack: verification failed\n");
        return 1;
    }
    return 0;
}

// Every file under dist/<page>, keyed by its path relative to that directory.
std::map<std::string, std::vector<uint8_t>> collectResources(const fs::path& pageDist) {
    std::map<std::string, std::vector<uint8_t>> resources;
    std::error_code ec;
    for (const auto& entry : fs::recursive_directory_iterator(pageDist, ec)) {
        if (!entry.is_regular_file()) continue;
        std::vector<uint8_t> bytes;
        if (readFile(entry.path(), bytes)) resources[fs::relative(entry.path(), pageDist, ec).generic_string()] = std::move(bytes);
    }
    return resources;
}

int pack(const std::string& projectArg, std::string outDir, const std::string& keyPath, bool publish) {
    const fs::path project = fs::absolute(projectArg);
    const fs::path dist = project / "dist";
    if (outDir.empty()) outDir = (project / "out").string();

    std::vector<uint8_t> manifestBytes;
    if (!readFile(project / "vpp.json", manifestBytes)) {
        std::printf("vpppack: cannot read %s\n"
                    "  create it with at least:  { \"id\": \"com.example.site\", \"name\": \"Site\", \"version\": \"1.0.0\" }\n",
                    (project / "vpp.json").string().c_str());
        return 1;
    }
    vpp::AppManifest manifest;
    std::string error;
    if (!vpp::parseAppManifest(std::string(manifestBytes.begin(), manifestBytes.end()), manifest, &error)) {
        std::printf("vpppack: %s\n", error.c_str());
        return 1;
    }

    vpp::SigningKey key;
    const bool signing = !keyPath.empty();
    if (signing && !vpp::loadSigningKey(keyPath, key, &error)) {
        std::printf("vpppack: %s\n", error.c_str());
        return 1;
    }
    if (publish && !signing) {
        std::printf("vpppack: --publish requires --key; the viewer never installs unsigned pages from the network\n");
        return 1;
    }

    // Pages: dist/<page>/dom.bin, or a legacy single dist/dom.bin as "index".
    std::vector<std::pair<std::string, fs::path>> pages;
    std::error_code ec;
    for (const auto& entry : fs::directory_iterator(dist, ec))
        if (entry.is_directory() && fs::exists(entry.path() / "dom.bin"))
            pages.push_back({entry.path().filename().string(), entry.path()});
    std::sort(pages.begin(), pages.end());
    if (pages.empty() && fs::exists(dist / "dom.bin")) pages.push_back({"index", dist});
    if (pages.empty()) {
        std::printf("vpppack: %s has no compiled pages; run vppc on the project first\n", dist.string().c_str());
        return 1;
    }

    std::printf("vpppack: %s %s (%s), %zu page%s\n", manifest.name.c_str(), manifest.version.c_str(), manifest.id.c_str(),
                pages.size(), pages.size() == 1 ? "" : "s");
    if (signing) std::printf("  signed by  %s\n", keyHex(key.publicKey).c_str());
    else std::printf("  UNSIGNED: the viewer will refuse these pages unless run with --allow-unsigned\n");

    size_t published = 0;
    for (const auto& [page, pageDist] : pages) {
        vpp::AppManifest pm = manifest;
        pm.page = page;
        const std::map<std::string, std::vector<uint8_t>> resources = collectResources(pageDist);
        const std::vector<uint8_t> file = vpp::Package::build(pm, resources, signing ? &key : nullptr);
        const fs::path out = fs::path(outDir) / (page + ".vpp");
        if (!writeFile(out, file)) {
            std::printf("vpppack: cannot write %s\n", out.string().c_str());
            return 1;
        }
        std::printf("  %-14s %zu resources, %zu bytes -> %s\n", (page + ".vpp").c_str(), resources.size(), file.size(),
                    out.string().c_str());

        if (publish) {
            vpp::Package pkg;
            if (!vpp::Package::open(file, pkg, &error)) {
                std::printf("vpppack: %s\n", error.c_str());
                return 1;
            }
            if (!writeFile(fs::path(outDir) / (page + ".vppm"), pkg.headerBytes())) {
                std::printf("vpppack: cannot write %s.vppm\n", page.c_str());
                return 1;
            }
            for (const vpp::PackageEntry& e : pkg.entries()) {
                const fs::path res = fs::path(outDir) / "res" / vpp::toHex(e.sha256);
                if (fs::exists(res)) continue;
                std::vector<uint8_t> data;
                if (!pkg.read(e.name, data, &error) || !writeFile(res, data)) {
                    std::printf("vpppack: cannot publish %s\n", e.name.c_str());
                    return 1;
                }
                ++published;
            }
        }
    }

    if (publish)
        std::printf("  published  %s: %zu manifests, %zu distinct resources in res/\n"
                    "             serve this directory and open a page's .vpp URL in the viewer\n",
                    outDir.c_str(), pages.size(), published);
    return 0;
}

} // namespace

int main(int argc, char** argv) {
    std::string project;
    std::string output;
    std::string inspectFile;
    std::string keyPath;
    bool publish = false;
    bool doKeygen = false;

    for (int i = 1; i < argc; ++i) {
        const std::string arg = argv[i];
        if (arg == "-o") {
            if (i + 1 >= argc) return usage();
            output = argv[++i];
        } else if (arg == "--publish" || arg == "-p") {
            publish = true;
        } else if (arg == "--key" || arg == "-k") {
            if (i + 1 >= argc) return usage();
            keyPath = argv[++i];
        } else if (arg == "--inspect" || arg == "-i") {
            if (i + 1 >= argc) return usage();
            inspectFile = argv[++i];
        } else if (arg == "keygen") {
            doKeygen = true;
        } else if (arg == "-h" || arg == "--help") {
            return usage();
        } else if (project.empty()) {
            project = arg;
        } else {
            return usage();
        }
    }

    if (doKeygen) return keygen(output);
    if (!inspectFile.empty()) return inspect(inspectFile);
    if (project.empty()) return usage();
    return pack(project, output, keyPath, publish);
}
