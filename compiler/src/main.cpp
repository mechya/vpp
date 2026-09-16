// vppc: the VPP compiler.
//
//   vppc <project dir> [-g]           compile every page of a site into dist/<page>/
//   vppc <page.html> [-g]             compile one page into <project>/dist/<page>/
//   vppc <app.js> [-o <file>] [-g]    compile one script to bytecode
//
// A site is a directory with vpp.json and pages/*.html (a lone index.html
// counts as the page "index"). Each page is expanded with its layout,
// includes, and components, then written as:
//
//   dist/<page>/dom.bin                 the finished page
//   dist/<page>/style/NN-<name>.bin     one per stylesheet, in load order
//   dist/<page>/code/NN-<name>.bin      one per script, in load order
//
// One resource per source file lets pages that share a stylesheet or script
// share the resource by hash.

#include "vpp/binary.h"
#include "vpp/script.h"
#include "vpp/template.h"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <sstream>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace {

int usage() {
    std::printf("usage: vppc <project dir> [-g]\n"
                "       vppc <page.html> [-g]\n"
                "       vppc <app.js> [-o <output.bin>] [-g]\n"
                "  -g  keep debug info in bytecode (line numbers in stack traces)\n");
    return 2;
}

bool readFile(const fs::path& path, std::string& out) {
    std::ifstream in(path, std::ios::binary);
    if (!in) return false;
    std::stringstream buffer;
    buffer << in.rdbuf();
    out = buffer.str();
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

std::string twoDigits(size_t n) {
    return (n < 10 ? "0" : "") + std::to_string(n);
}

int compileScript(const std::string& input, std::string output, bool keepDebugInfo) {
    if (output.empty()) output = (fs::path(input).parent_path() / "code.bin").string();
    std::string source;
    if (!readFile(input, source)) {
        std::printf("vppc: cannot read %s\n", input.c_str());
        return 1;
    }
    std::vector<uint8_t> bytecode;
    std::string error;
    if (!vpp::ScriptHost::compile(source, fs::path(input).filename().string(), bytecode, &error, keepDebugInfo)) {
        std::printf("vppc: %s\n%s\n", input.c_str(), error.c_str());
        return 1;
    }
    if (!writeFile(output, bytecode)) {
        std::printf("vppc: cannot write %s\n", output.c_str());
        return 1;
    }
    std::printf("vppc: %s -> %s (%zu bytes bytecode)\n", input.c_str(), output.c_str(), bytecode.size());
    return 0;
}

bool compilePage(const fs::path& root, const fs::path& pageFile, const fs::path& distRoot,
                 const vpp::TemplateOptions& options, bool keepDebugInfo) {
    const std::string page = pageFile.stem().string();
    const fs::path dist = distRoot / page;

    vpp::ExpandedPage expanded;
    std::string error;
    if (!vpp::expandPage(pageFile, options, expanded, &error)) {
        std::printf("vppc: %s\n", error.c_str());
        return false;
    }
    for (const std::string& w : expanded.warnings) std::printf("  warning  %s\n", w.c_str());

    std::error_code ec;
    fs::remove_all(dist, ec);

    const std::vector<uint8_t> dom = vpp::encodeDom(*expanded.document);
    if (!writeFile(dist / "dom.bin", dom)) {
        std::printf("vppc: cannot write %s\n", (dist / "dom.bin").string().c_str());
        return false;
    }
    std::printf("  page     %-28s dom.bin %zu bytes\n", (fs::relative(pageFile, root, ec).generic_string()).c_str(),
                dom.size());

    for (size_t i = 0; i < expanded.styles.size(); ++i) {
        const vpp::PageStyle& s = expanded.styles[i];
        const std::string name = "style/" + twoDigits(i) + "-" + s.name + ".bin";
        const std::vector<uint8_t> bytes = vpp::encodeStyleSheet(s.sheet);
        if (!writeFile(dist / name, bytes)) {
            std::printf("vppc: cannot write %s\n", name.c_str());
            return false;
        }
        std::printf("    %-30s %zu rules, %zu bytes\n", name.c_str(), s.sheet.rules.size(), bytes.size());
    }

    for (size_t i = 0; i < expanded.scripts.size(); ++i) {
        const vpp::PageScript& s = expanded.scripts[i];
        const std::string name = "code/" + twoDigits(i) + "-" + s.name + ".bin";
        std::vector<uint8_t> bytecode;
        const std::string filename = s.path.empty() ? s.name + ".js" : s.path.filename().string();
        if (!vpp::ScriptHost::compile(s.source, filename, bytecode, &error, keepDebugInfo)) {
            std::printf("vppc: %s\n%s\n", filename.c_str(), error.c_str());
            return false;
        }
        if (!writeFile(dist / name, bytecode)) {
            std::printf("vppc: cannot write %s\n", name.c_str());
            return false;
        }
        std::printf("    %-30s %zu bytes\n", name.c_str(), bytecode.size());
    }
    return true;
}

vpp::TemplateOptions optionsFor(const fs::path& root) {
    vpp::TemplateOptions options;
    options.projectRoot = root;
    std::string json;
    if (readFile(root / "vpp.json", json)) options.componentAliases = vpp::parseComponentAliases(json);
    return options;
}

int compileProject(const std::string& dirArg, bool keepDebugInfo) {
    const fs::path root = fs::absolute(dirArg);
    std::vector<fs::path> pages;
    std::error_code ec;
    if (fs::is_directory(root / "pages", ec)) {
        for (const auto& entry : fs::directory_iterator(root / "pages", ec)) {
            const std::string ext = entry.path().extension().string();
            if (entry.is_regular_file() && (ext == ".html" || ext == ".htm")) pages.push_back(entry.path());
        }
        std::sort(pages.begin(), pages.end());
    } else if (fs::exists(root / "index.html")) {
        pages.push_back(root / "index.html");
    }
    if (pages.empty()) {
        std::printf("vppc: %s has no pages/*.html and no index.html\n", root.string().c_str());
        return 1;
    }

    std::printf("vppc: %s (%zu page%s)\n", root.string().c_str(), pages.size(), pages.size() == 1 ? "" : "s");
    const vpp::TemplateOptions options = optionsFor(root);
    for (const fs::path& page : pages)
        if (!compilePage(root, page, root / "dist", options, keepDebugInfo)) return 1;
    std::printf("vppc: written to %s\n", (root / "dist").string().c_str());
    return 0;
}

int compileSinglePage(const std::string& pageArg, bool keepDebugInfo) {
    const fs::path page = fs::absolute(pageArg);
    const fs::path root = vpp::findProjectRoot(page);
    std::printf("vppc: %s (project %s)\n", page.string().c_str(), root.string().c_str());
    if (!compilePage(root, page, root / "dist", optionsFor(root), keepDebugInfo)) return 1;
    std::printf("vppc: written to %s\n", (root / "dist" / page.stem()).string().c_str());
    return 0;
}

} // namespace

int main(int argc, char** argv) {
    std::string input;
    std::string output;
    bool keepDebugInfo = false;

    for (int i = 1; i < argc; ++i) {
        const std::string arg = argv[i];
        if (arg == "-o") {
            if (i + 1 >= argc) return usage();
            output = argv[++i];
        } else if (arg == "-g") {
            keepDebugInfo = true;
        } else if (arg == "-h" || arg == "--help") {
            return usage();
        } else if (input.empty()) {
            input = arg;
        } else {
            return usage();
        }
    }
    if (input.empty()) return usage();

    std::error_code ec;
    if (fs::is_directory(input, ec)) return compileProject(input, keepDebugInfo);

    std::string ext = fs::path(input).extension().string();
    for (char& c : ext) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    if (ext == ".html" || ext == ".htm") return compileSinglePage(input, keepDebugInfo);
    if (ext == ".js" || ext == ".mjs") return compileScript(input, output, keepDebugInfo);

    std::printf("vppc: %s: expected a project directory, an .html page, or a .js script\n", input.c_str());
    return usage();
}
