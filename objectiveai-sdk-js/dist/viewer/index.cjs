'use strict';

var zod = require('zod');

var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var init_tslib_es6 = __esm({
  "../node_modules/@tauri-apps/api/external/tslib/tslib.es6.js"() {
  }
});

// ../node_modules/@tauri-apps/api/core.js
function transformCallback(callback, once2 = false) {
  return window.__TAURI_INTERNALS__.transformCallback(callback, once2);
}
async function invoke(cmd, args = {}, options) {
  return window.__TAURI_INTERNALS__.invoke(cmd, args, options);
}
var init_core = __esm({
  "../node_modules/@tauri-apps/api/core.js"() {
    init_tslib_es6();
  }
});

// ../node_modules/@tauri-apps/api/event.js
var event_exports = {};
__export(event_exports, {
  TauriEvent: () => TauriEvent,
  emit: () => emit,
  emitTo: () => emitTo,
  listen: () => listen,
  once: () => once
});
async function _unlisten(event, eventId) {
  window.__TAURI_EVENT_PLUGIN_INTERNALS__.unregisterListener(event, eventId);
  await invoke("plugin:event|unlisten", {
    event,
    eventId
  });
}
async function listen(event, handler, options) {
  var _a;
  const target = typeof (options === null || options === void 0 ? void 0 : options.target) === "string" ? { kind: "AnyLabel", label: options.target } : (_a = options === null || options === void 0 ? void 0 : options.target) !== null && _a !== void 0 ? _a : { kind: "Any" };
  return invoke("plugin:event|listen", {
    event,
    target,
    handler: transformCallback(handler)
  }).then((eventId) => {
    return async () => _unlisten(event, eventId);
  });
}
async function once(event, handler, options) {
  return listen(event, (eventData) => {
    void _unlisten(event, eventData.id);
    handler(eventData);
  }, options);
}
async function emit(event, payload) {
  await invoke("plugin:event|emit", {
    event,
    payload
  });
}
async function emitTo(target, event, payload) {
  const eventTarget = typeof target === "string" ? { kind: "AnyLabel", label: target } : target;
  await invoke("plugin:event|emit_to", {
    target: eventTarget,
    event,
    payload
  });
}
var TauriEvent;
var init_event = __esm({
  "../node_modules/@tauri-apps/api/event.js"() {
    init_core();
    (function(TauriEvent2) {
      TauriEvent2["WINDOW_RESIZED"] = "tauri://resize";
      TauriEvent2["WINDOW_MOVED"] = "tauri://move";
      TauriEvent2["WINDOW_CLOSE_REQUESTED"] = "tauri://close-requested";
      TauriEvent2["WINDOW_DESTROYED"] = "tauri://destroyed";
      TauriEvent2["WINDOW_FOCUS"] = "tauri://focus";
      TauriEvent2["WINDOW_BLUR"] = "tauri://blur";
      TauriEvent2["WINDOW_SCALE_FACTOR_CHANGED"] = "tauri://scale-change";
      TauriEvent2["WINDOW_THEME_CHANGED"] = "tauri://theme-changed";
      TauriEvent2["WINDOW_CREATED"] = "tauri://window-created";
      TauriEvent2["WEBVIEW_CREATED"] = "tauri://webview-created";
      TauriEvent2["DRAG_ENTER"] = "tauri://drag-enter";
      TauriEvent2["DRAG_OVER"] = "tauri://drag-over";
      TauriEvent2["DRAG_DROP"] = "tauri://drag-drop";
      TauriEvent2["DRAG_LEAVE"] = "tauri://drag-leave";
    })(TauriEvent || (TauriEvent = {}));
  }
});
var JsonValueSchema = zod.z.union([
  zod.z.string(),
  zod.z.number(),
  zod.z.boolean(),
  zod.z.null(),
  zod.z.array(zod.z.lazy(() => JsonValueSchema)),
  zod.z.record(zod.z.string(), zod.z.lazy(() => JsonValueSchema))
]);

// src/viewer/event.ts
var ViewerEventSchema = zod.z.union([zod.z.object({
  destination: zod.z.string(),
  sub_type: zod.z.string(),
  type: zod.z.literal("inbound"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (e.g. `daemon` for raw daemon-broadcast frames on\nthe `"objectiveai"` channel; the JS bridge repackages routed\n`plugins/run` frames as `plugins_run` for plugin iframes).').meta({ "variantTitle": "Inbound" }), zod.z.object({
  destination: zod.z.string(),
  type: zod.z.literal("cli_command"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. One stdout JSONL line from an objectiveai cli\nbinary the host spawned for an `invokeCli` this iframe\nstarted, terminated by a synthetic `{"type":"end"}` line. No\nsub_type \u2014 a single invocation produces a single stream of\nlines.').meta({ "variantTitle": "CliCommand" })]).describe("Every event the viewer emits to the JS side. Serde-tagged on\n`type` so the JS bridge can pattern-match and decide how to\nrepackage each variant for the destination iframe.\n\n`destination` is `\"objectiveai\"` for built-in events, or the\nplugin's repository name otherwise. For `CliCommand` it's the\nrepository name of whichever iframe invoked the CLI \u2014 the bridge\nderives it from `MessageEvent.source`, the plugin author never\nsets it.").meta({ title: "viewer.Event" });

// src/viewer/index.ts
function isInIframe() {
  return typeof window !== "undefined" && window.parent !== window;
}
var inboundListeners = /* @__PURE__ */ new Map();
var inboundHandlerAttached = false;
function attachInboundHandler() {
  if (inboundHandlerAttached) return;
  if (typeof window === "undefined") return;
  window.addEventListener("message", (event) => {
    const msg = event.data;
    if (!msg || typeof msg !== "object") return;
    if (msg.kind !== "plugin-event") return;
    if (msg.type !== "inbound") return;
    const set = inboundListeners.get(msg.sub_type);
    if (!set) return;
    for (const fn of set) {
      try {
        fn(msg.value);
      } catch (e) {
        console.error("@objectiveai/sdk/viewer listener threw:", e);
      }
    }
  });
  inboundHandlerAttached = true;
}
function listen2(sub_type, handler) {
  if (!isInIframe()) {
    let unlisten = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await Promise.resolve().then(() => (init_event(), event_exports));
        if (cancelled) return;
        const u = await mod.listen(
          `plugin-${sub_type}`,
          (e) => handler(e.payload?.value)
        );
        if (cancelled) {
          u();
        } else {
          unlisten = u;
        }
      } catch {
        console.warn(
          `@objectiveai/sdk/viewer: listen('${sub_type}') called outside an iframe and @tauri-apps/api is unavailable; events will not fire.`
        );
      }
    })();
    return () => {
      cancelled = true;
      if (unlisten) unlisten();
    };
  }
  attachInboundHandler();
  const set = inboundListeners.get(sub_type) ?? /* @__PURE__ */ new Set();
  const fn = (value) => handler(value);
  set.add(fn);
  inboundListeners.set(sub_type, set);
  return () => {
    const s = inboundListeners.get(sub_type);
    if (!s) return;
    s.delete(fn);
    if (s.size === 0) inboundListeners.delete(sub_type);
  };
}
function __resetForTests() {
  inboundListeners.clear();
}

exports.ViewerEventSchema = ViewerEventSchema;
exports.__resetForTests = __resetForTests;
exports.listen = listen2;
