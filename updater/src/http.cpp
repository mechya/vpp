// HTTP client. On Windows this uses WinHTTP, which handles TLS and system
// proxy settings. Other platforms are not implemented yet.

#include "vpp/http.h"

#ifdef _WIN32
#include <windows.h>
#include <winhttp.h>
#endif

namespace vpp {

bool isRemoteUrl(const std::string& s) {
    return s.rfind("http://", 0) == 0 || s.rfind("https://", 0) == 0 || s.rfind("vpp://", 0) == 0;
}

#ifdef _WIN32

namespace {

std::wstring widen(const std::string& s) {
    if (s.empty()) return std::wstring();
    const int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    std::wstring out(static_cast<size_t>(n), L'\0');
    MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()), &out[0], n);
    return out;
}

std::string lastError(const char* what) {
    return std::string(what) + " failed (error " + std::to_string(GetLastError()) + ")";
}

struct Handle {
    HINTERNET h = nullptr;
    ~Handle() {
        if (h) WinHttpCloseHandle(h);
    }
};

} // namespace

bool httpGet(const std::string& urlIn, std::vector<uint8_t>& out, std::string* error) {
    std::string url = urlIn;
    if (url.rfind("vpp://", 0) == 0) url = "https://" + url.substr(6);

    const std::wstring wide = widen(url);
    URL_COMPONENTS parts{};
    parts.dwStructSize = sizeof(parts);
    wchar_t host[256] = {};
    wchar_t path[2048] = {};
    parts.lpszHostName = host;
    parts.dwHostNameLength = 256;
    parts.lpszUrlPath = path;
    parts.dwUrlPathLength = 2048;
    if (!WinHttpCrackUrl(wide.c_str(), 0, 0, &parts)) {
        if (error) *error = "invalid URL " + url;
        return false;
    }
    const bool secure = parts.nScheme == INTERNET_SCHEME_HTTPS;

    Handle session;
    // DEFAULT_PROXY honours the system proxy settings without WPAD discovery,
    // which can stall for many seconds on networks without a proxy.
    session.h = WinHttpOpen(L"VPP Viewer/0.1", WINHTTP_ACCESS_TYPE_DEFAULT_PROXY, WINHTTP_NO_PROXY_NAME,
                            WINHTTP_NO_PROXY_BYPASS, 0);
    if (session.h) {
        // Connect and receive within 10 s; a dead server should fail fast so
        // the installed copy can run offline.
        WinHttpSetTimeouts(session.h, 5000, 10000, 10000, 30000);
    }
    if (!session.h) {
        if (error) *error = lastError("WinHttpOpen");
        return false;
    }

    Handle connection;
    connection.h = WinHttpConnect(session.h, host, parts.nPort, 0);
    if (!connection.h) {
        if (error) *error = lastError("WinHttpConnect");
        return false;
    }

    Handle request;
    request.h = WinHttpOpenRequest(connection.h, L"GET", path, nullptr, WINHTTP_NO_REFERER,
                                   WINHTTP_DEFAULT_ACCEPT_TYPES, secure ? WINHTTP_FLAG_SECURE : 0);
    if (!request.h) {
        if (error) *error = lastError("WinHttpOpenRequest");
        return false;
    }

    if (!WinHttpSendRequest(request.h, WINHTTP_NO_ADDITIONAL_HEADERS, 0, WINHTTP_NO_REQUEST_DATA, 0, 0, 0) ||
        !WinHttpReceiveResponse(request.h, nullptr)) {
        if (error) *error = "cannot reach " + url + " (error " + std::to_string(GetLastError()) + ")";
        return false;
    }

    DWORD status = 0;
    DWORD statusSize = sizeof(status);
    WinHttpQueryHeaders(request.h, WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                        WINHTTP_HEADER_NAME_BY_INDEX, &status, &statusSize, WINHTTP_NO_HEADER_INDEX);
    if (status != 200) {
        if (error) *error = "HTTP " + std::to_string(status) + " for " + url;
        return false;
    }

    out.clear();
    for (;;) {
        DWORD available = 0;
        if (!WinHttpQueryDataAvailable(request.h, &available)) {
            if (error) *error = lastError("WinHttpQueryDataAvailable");
            return false;
        }
        if (available == 0) break;
        const size_t start = out.size();
        out.resize(start + available);
        DWORD read = 0;
        if (!WinHttpReadData(request.h, out.data() + start, available, &read)) {
            if (error) *error = lastError("WinHttpReadData");
            return false;
        }
        out.resize(start + read);
        if (read == 0) break;
    }
    return true;
}

#else

bool httpGet(const std::string& url, std::vector<uint8_t>&, std::string* error) {
    if (error) *error = "networking is not implemented on this platform yet (" + url + ")";
    return false;
}

#endif

} // namespace vpp
