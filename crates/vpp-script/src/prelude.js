// The page's JavaScript API, built in JavaScript on top of the small native
// interface in bindings.rs. It runs once, before any page script, and returns
// the function Rust calls to dispatch events.
//
// What pages get (docs/reference/viewer.md, "What the engine does"):
//   document.getElementById(id)
//   element.id, element.tagName, element.textContent (read and write),
//   element.addEventListener(type, listener)
//   console.log(...values)
//   VPP.window.popup(message), .close(), .minimize(), .maximize()
//
// Elements are identified by numeric handles that bindings.rs checks on every
// use, so a page can never reach anything but its own document.
(function (native) {
  "use strict";

  const listeners = new Map(); // handle -> [{ type, fn }]
  const wrappers = new Map();  // handle -> Element, so each element has one object

  class Element {
    #handle;

    constructor(handle) {
      this.#handle = handle;
    }

    get id() {
      return native.id(this.#handle);
    }

    get tagName() {
      return native.tagName(this.#handle);
    }

    get textContent() {
      return native.getText(this.#handle);
    }

    set textContent(value) {
      native.setText(this.#handle, String(value ?? ""));
    }

    addEventListener(type, listener) {
      if (typeof listener !== "function") {
        throw new TypeError("addEventListener(type, listener) expects a function");
      }
      let list = listeners.get(this.#handle);
      if (!list) {
        list = [];
        listeners.set(this.#handle, list);
      }
      list.push({ type: String(type), fn: listener });
    }
  }

  function wrap(handle) {
    if (handle === null || handle === undefined) {
      return null;
    }
    let element = wrappers.get(handle);
    if (!element) {
      element = new Element(handle);
      wrappers.set(handle, element);
    }
    return element;
  }

  globalThis.document = {
    getElementById(id) {
      return wrap(native.byId(String(id)));
    },
  };

  globalThis.console = {
    log(...values) {
      native.log(values.map((v) => String(v)).join(" "));
    },
  };

  globalThis.VPP = {
    window: {
      popup(message) {
        native.popup(String(message ?? ""));
      },
      close() {
        native.close();
      },
      minimize() {
        native.minimize();
      },
      maximize() {
        native.maximize();
      },
    },
  };

  // Calls the listeners for `type` on each element of `chain`: the target
  // first, then its ancestors. A listener that throws is logged and the
  // others still run, as in a browser.
  return function dispatch(type, chain) {
    const event = { type, target: wrap(chain[0]) };
    for (const handle of chain) {
      const list = listeners.get(handle);
      if (!list) {
        continue;
      }
      // A copy: a listener may add listeners while running.
      for (const { type: wanted, fn } of [...list]) {
        if (wanted !== type) {
          continue;
        }
        try {
          fn.call(wrap(handle), event);
        } catch (error) {
          const stack = error && error.stack ? "\n" + error.stack : "";
          native.log("uncaught: " + String(error) + stack);
        }
      }
    }
  };
});
