// JavaScript bindings. quickjs-ng executes the code; this file exposes VPP's
// DOM and host services to it. Nothing outside this file includes quickjs.

#include "vpp/script.h"

#include <quickjs.h>

#include <algorithm>
#include <cctype>
#include <map>
#include <string>
#include <vector>

namespace vpp {

namespace {

std::string exceptionToString(JSContext* ctx) {
    JSValue exc = JS_GetException(ctx);
    std::string out;
    if (const char* msg = JS_ToCString(ctx, exc)) {
        out = msg;
        JS_FreeCString(ctx, msg);
    }
    JSValue stack = JS_GetPropertyStr(ctx, exc, "stack");
    if (!JS_IsUndefined(stack)) {
        if (const char* s = JS_ToCString(ctx, stack)) {
            if (*s) {
                out += "\n";
                out += s;
            }
            JS_FreeCString(ctx, s);
        }
    }
    JS_FreeValue(ctx, stack);
    JS_FreeValue(ctx, exc);
    return out;
}

std::string toStdString(JSContext* ctx, JSValueConst v) {
    const char* s = JS_ToCString(ctx, v);
    if (!s) return std::string();
    std::string out(s);
    JS_FreeCString(ctx, s);
    return out;
}

void runPendingJobs(JSRuntime* rt) {
    JSContext* jobCtx = nullptr;
    while (JS_ExecutePendingJob(rt, &jobCtx) > 0) {
    }
}

struct Listener {
    std::string type;
    JSValue fn;
};

} // namespace

struct ScriptHost::Impl {
    Node& document;
    ScriptCallbacks callbacks;
    JSRuntime* rt = nullptr;
    JSContext* ctx = nullptr;
    JSClassID elementClass = 0;
    std::map<Node*, JSValue> wrappers;
    std::map<const Node*, std::vector<Listener>> listeners;

    Impl(Node& doc, ScriptCallbacks cb) : document(doc), callbacks(std::move(cb)) {}

    static Impl* from(JSContext* c) { return static_cast<Impl*>(JS_GetContextOpaque(c)); }

    // One JS object per element, created on first use, kept alive for the host's lifetime.
    JSValue wrap(Node* node) {
        auto it = wrappers.find(node);
        if (it != wrappers.end()) return JS_DupValue(ctx, it->second);
        JSValue obj = JS_NewObjectClass(ctx, elementClass);
        JS_SetOpaque(obj, node);
        wrappers.emplace(node, JS_DupValue(ctx, obj));
        return obj;
    }

    Node* unwrap(JSValueConst v) { return static_cast<Node*>(JS_GetOpaque2(ctx, v, elementClass)); }

    void setMethod(JSValueConst obj, const char* name, JSCFunction* fn, int length) {
        JS_SetPropertyStr(ctx, obj, name, JS_NewCFunction(ctx, fn, name, length));
    }

    void setAccessor(JSValueConst obj, const char* name, JSCFunction* getter, JSCFunction* setter) {
        JSAtom atom = JS_NewAtom(ctx, name);
        JSValue g = JS_NewCFunction(ctx, getter, name, 0);
        JSValue s = setter ? JS_NewCFunction(ctx, setter, name, 1) : JS_UNDEFINED;
        JS_DefinePropertyGetSet(ctx, obj, atom, g, s,
                                JS_PROP_CONFIGURABLE | JS_PROP_ENUMERABLE | JS_PROP_HAS_CONFIGURABLE |
                                    JS_PROP_HAS_ENUMERABLE);
        JS_FreeAtom(ctx, atom);
    }

    // --- document -----------------------------------------------------------

    static JSValue getElementById(JSContext* c, JSValueConst, int argc, JSValueConst* argv) {
        Impl* self = from(c);
        if (argc < 1) return JS_NULL;
        Node* node = self->document.findById(toStdString(c, argv[0]));
        return node ? self->wrap(node) : JS_NULL;
    }

    // --- Element -------------------------------------------------------------

    static JSValue addEventListener(JSContext* c, JSValueConst thisVal, int argc, JSValueConst* argv) {
        Impl* self = from(c);
        Node* node = self->unwrap(thisVal);
        if (!node) return JS_EXCEPTION;
        if (argc < 2 || !JS_IsFunction(c, argv[1]))
            return JS_ThrowTypeError(c, "addEventListener(type, listener) expects a function");
        self->listeners[node].push_back({toStdString(c, argv[0]), JS_DupValue(c, argv[1])});
        return JS_UNDEFINED;
    }

    static JSValue getTextContent(JSContext* c, JSValueConst thisVal, int, JSValueConst*) {
        Node* node = from(c)->unwrap(thisVal);
        if (!node) return JS_EXCEPTION;
        return JS_NewString(c, node->textContent().c_str());
    }

    static JSValue setTextContent(JSContext* c, JSValueConst thisVal, int argc, JSValueConst* argv) {
        Impl* self = from(c);
        Node* node = self->unwrap(thisVal);
        if (!node) return JS_EXCEPTION;
        node->setTextContent(argc > 0 ? toStdString(c, argv[0]) : std::string());
        if (self->callbacks.invalidate) self->callbacks.invalidate();
        return JS_UNDEFINED;
    }

    static JSValue getId(JSContext* c, JSValueConst thisVal, int, JSValueConst*) {
        Node* node = from(c)->unwrap(thisVal);
        if (!node) return JS_EXCEPTION;
        return JS_NewString(c, node->id().c_str());
    }

    static JSValue getTagName(JSContext* c, JSValueConst thisVal, int, JSValueConst*) {
        Node* node = from(c)->unwrap(thisVal);
        if (!node) return JS_EXCEPTION;
        std::string tag = node->tag();
        std::transform(tag.begin(), tag.end(), tag.begin(),
                       [](unsigned char ch) { return static_cast<char>(std::toupper(ch)); });
        return JS_NewString(c, tag.c_str());
    }

    // --- console / VPP ---------------------------------------------------------

    static JSValue consoleLog(JSContext* c, JSValueConst, int argc, JSValueConst* argv) {
        Impl* self = from(c);
        std::string line;
        for (int i = 0; i < argc; ++i) {
            if (i) line += ' ';
            line += toStdString(c, argv[i]);
        }
        if (self->callbacks.log) self->callbacks.log(line);
        return JS_UNDEFINED;
    }

    static JSValue windowPopup(JSContext* c, JSValueConst, int argc, JSValueConst* argv) {
        Impl* self = from(c);
        if (self->callbacks.popup) self->callbacks.popup(argc > 0 ? toStdString(c, argv[0]) : std::string());
        return JS_UNDEFINED;
    }

    static JSValue windowClose(JSContext* c, JSValueConst, int, JSValueConst*) {
        Impl* self = from(c);
        if (self->callbacks.close) self->callbacks.close();
        return JS_UNDEFINED;
    }

    static JSValue windowMinimize(JSContext* c, JSValueConst, int, JSValueConst*) {
        Impl* self = from(c);
        if (self->callbacks.minimize) self->callbacks.minimize();
        return JS_UNDEFINED;
    }

    static JSValue windowMaximize(JSContext* c, JSValueConst, int, JSValueConst*) {
        Impl* self = from(c);
        if (self->callbacks.maximize) self->callbacks.maximize();
        return JS_UNDEFINED;
    }

    // --- setup -----------------------------------------------------------------

    void installGlobals() {
        JS_NewClassID(rt, &elementClass);
        JSClassDef def{};
        def.class_name = "Element";
        JS_NewClass(rt, elementClass, &def);

        JSValue proto = JS_NewObject(ctx);
        setMethod(proto, "addEventListener", addEventListener, 2);
        setAccessor(proto, "textContent", getTextContent, setTextContent);
        setAccessor(proto, "id", getId, nullptr);
        setAccessor(proto, "tagName", getTagName, nullptr);
        JS_SetClassProto(ctx, elementClass, proto);

        JSValue global = JS_GetGlobalObject(ctx);

        JSValue doc = JS_NewObject(ctx);
        setMethod(doc, "getElementById", getElementById, 1);
        JS_SetPropertyStr(ctx, global, "document", doc);

        JSValue console = JS_NewObject(ctx);
        setMethod(console, "log", consoleLog, 1);
        JS_SetPropertyStr(ctx, global, "console", console);

        JSValue window = JS_NewObject(ctx);
        setMethod(window, "popup", windowPopup, 1);
        setMethod(window, "close", windowClose, 0);
        setMethod(window, "minimize", windowMinimize, 0);
        setMethod(window, "maximize", windowMaximize, 0);
        JSValue vpp = JS_NewObject(ctx);
        JS_SetPropertyStr(ctx, vpp, "window", window);
        JS_SetPropertyStr(ctx, global, "VPP", vpp);

        JS_FreeValue(ctx, global);
    }

    bool finish(JSValue result, std::string* error) {
        if (JS_IsException(result)) {
            if (error) *error = exceptionToString(ctx);
            return false;
        }
        JS_FreeValue(ctx, result);
        runPendingJobs(rt);
        return true;
    }
};

ScriptHost::ScriptHost(Node& document, ScriptCallbacks callbacks)
    : impl_(new Impl(document, std::move(callbacks))) {
    impl_->rt = JS_NewRuntime();
    impl_->ctx = JS_NewContext(impl_->rt);
    JS_SetContextOpaque(impl_->ctx, impl_.get());
    impl_->installGlobals();
}

ScriptHost::~ScriptHost() {
    for (auto& entry : impl_->listeners)
        for (Listener& l : entry.second)
            JS_FreeValue(impl_->ctx, l.fn);
    for (auto& entry : impl_->wrappers)
        JS_FreeValue(impl_->ctx, entry.second);
    JS_FreeContext(impl_->ctx);
    JS_FreeRuntime(impl_->rt);
}

bool ScriptHost::runSource(const std::string& source, const std::string& filename, std::string* error) {
    JSValue result = JS_Eval(impl_->ctx, source.c_str(), source.size(), filename.c_str(), JS_EVAL_TYPE_GLOBAL);
    return impl_->finish(result, error);
}

bool ScriptHost::runBytecode(const std::vector<uint8_t>& bytecode, std::string* error) {
    JSValue fn = JS_ReadObject(impl_->ctx, bytecode.data(), bytecode.size(), JS_READ_OBJ_BYTECODE);
    if (JS_IsException(fn)) {
        if (error) *error = "invalid bytecode: " + exceptionToString(impl_->ctx);
        return false;
    }
    return impl_->finish(JS_EvalFunction(impl_->ctx, fn), error);
}

void ScriptHost::dispatchClick(const Node& target) {
    Impl& im = *impl_;
    const Node* element = target.isElement() ? &target : target.parent();
    if (!element) return;

    JSValue event = JS_NewObject(im.ctx);
    JS_SetPropertyStr(im.ctx, event, "type", JS_NewString(im.ctx, "click"));
    JS_SetPropertyStr(im.ctx, event, "target", im.wrap(const_cast<Node*>(element)));

    for (const Node* n = element; n; n = n->parent()) {
        auto it = im.listeners.find(n);
        if (it == im.listeners.end()) continue;

        // Copy: a listener may register more listeners while running.
        std::vector<Listener> current = it->second;
        for (const Listener& l : current) {
            if (l.type != "click") continue;
            JSValue self = im.wrap(const_cast<Node*>(n));
            JSValue result = JS_Call(im.ctx, l.fn, self, 1, &event);
            JS_FreeValue(im.ctx, self);
            if (JS_IsException(result)) {
                const std::string message = exceptionToString(im.ctx);
                if (im.callbacks.log) im.callbacks.log("uncaught: " + message);
            } else {
                JS_FreeValue(im.ctx, result);
            }
        }
    }

    JS_FreeValue(im.ctx, event);
    runPendingJobs(im.rt);
}

bool ScriptHost::compile(const std::string& source, const std::string& filename, std::vector<uint8_t>& out,
                         std::string* error, bool keepDebugInfo) {
    JSRuntime* rt = JS_NewRuntime();
    JSContext* ctx = JS_NewContext(rt);
    bool ok = false;

    JSValue fn = JS_Eval(ctx, source.c_str(), source.size(), filename.c_str(),
                         JS_EVAL_TYPE_GLOBAL | JS_EVAL_FLAG_COMPILE_ONLY);
    if (JS_IsException(fn)) {
        if (error) *error = exceptionToString(ctx);
    } else {
        int flags = JS_WRITE_OBJ_BYTECODE | JS_WRITE_OBJ_STRIP_SOURCE;
        if (!keepDebugInfo) flags |= JS_WRITE_OBJ_STRIP_DEBUG;
        size_t size = 0;
        uint8_t* bytes = JS_WriteObject(ctx, &size, fn, flags);
        if (bytes) {
            out.assign(bytes, bytes + size);
            js_free(ctx, bytes);
            ok = true;
        } else if (error) {
            *error = exceptionToString(ctx);
        }
        JS_FreeValue(ctx, fn);
    }

    JS_FreeContext(ctx);
    JS_FreeRuntime(rt);
    return ok;
}

} // namespace vpp
