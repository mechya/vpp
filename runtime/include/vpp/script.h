#pragma once

#include "vpp/dom.h"

#include <cstdint>
#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace vpp {

// Services the host application provides to scripts.
struct ScriptCallbacks {
    std::function<void(const std::string&)> log;    // console.log
    std::function<void(const std::string&)> popup;  // VPP.window.popup(message)
    std::function<void()> close;                    // VPP.window.close()
    std::function<void()> minimize;                 // VPP.window.minimize()
    std::function<void()> maximize;                 // VPP.window.maximize() (toggles restore)
    std::function<void()> invalidate;               // the DOM changed; layout must run again
};

// Owns a JavaScript runtime bound to one document. Exposes a small DOM API
// (document.getElementById, addEventListener, textContent, id, tagName),
// console.log, and the VPP.window namespace.
class ScriptHost {
public:
    ScriptHost(Node& document, ScriptCallbacks callbacks);
    ~ScriptHost();

    ScriptHost(const ScriptHost&) = delete;
    ScriptHost& operator=(const ScriptHost&) = delete;

    // Development mode: run JavaScript source text.
    bool runSource(const std::string& source, const std::string& filename, std::string* error);

    // Release mode: run VPP bytecode produced by compile().
    bool runBytecode(const std::vector<uint8_t>& bytecode, std::string* error);

    // Fires a click event at target and lets it bubble to ancestors.
    void dispatchClick(const Node& target);

    // Compiles source to bytecode. Needs no document; used by the compiler tool.
    // The source text is never written. Debug info (line numbers for stack
    // traces) is stripped unless keepDebugInfo is set.
    static bool compile(const std::string& source, const std::string& filename,
                        std::vector<uint8_t>& out, std::string* error, bool keepDebugInfo = false);

private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

} // namespace vpp
