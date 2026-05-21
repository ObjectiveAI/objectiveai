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

// src/error.ts
var ObjectiveAIFetchError = class extends Error {
  constructor(codeOrBody, rawBody) {
    let body;
    if (typeof codeOrBody !== "number") {
      body = codeOrBody;
    } else if (rawBody === null || rawBody === void 0) {
      body = { code: codeOrBody, message: null };
    } else {
      let parsed;
      try {
        parsed = JSON.parse(rawBody);
      } catch {
        body = { code: codeOrBody, message: rawBody };
        super(JSON.stringify(body));
        this.name = "ObjectiveAIFetchError";
        this.body = body;
        return;
      }
      if (isResponseError(parsed)) {
        body = parsed;
      } else {
        body = { code: codeOrBody, message: parsed };
      }
    }
    super(JSON.stringify(body));
    this.name = "ObjectiveAIFetchError";
    this.body = body;
  }
  /**
   * Convenience getter for the error code.
   */
  get code() {
    return this.body.code;
  }
  /**
   * Serialize to ErrorResponseError JSON format.
   */
  toJSON() {
    return this.body;
  }
};
function isResponseError(obj) {
  return typeof obj === "object" && obj !== null && "code" in obj && typeof obj.code === "number" && "message" in obj;
}

// src/stream.ts
var Stream = class _Stream {
  constructor(response, controller) {
    this.buffer = "";
    this.done = false;
    if (!response.body) {
      throw new Error("Response body is null");
    }
    this.reader = response.body.getReader();
    this.decoder = new TextDecoder();
    this.controller = controller ?? null;
    this.asyncIterableSource = null;
  }
  /**
   * Wrap an `AsyncIterable<T>` (e.g. the viewer-mode api-call
   * iterator) as a `Stream<T>`. Skips the SSE parsing path entirely;
   * the iterator yields chunks pre-parsed by its source.
   */
  static fromAsyncIterable(source, controller) {
    const stream = Object.create(_Stream.prototype);
    stream.reader = null;
    stream.decoder = new TextDecoder();
    stream.buffer = "";
    stream.done = false;
    stream.controller = controller ?? null;
    stream.asyncIterableSource = source;
    return stream;
  }
  /**
   * Abort the stream.
   */
  abort() {
    this.controller?.abort();
  }
  async *[Symbol.asyncIterator]() {
    if (this.asyncIterableSource) {
      for await (const chunk of this.asyncIterableSource) {
        yield chunk;
      }
      return;
    }
    if (!this.reader) {
      throw new Error("Stream has no backing source");
    }
    const reader = this.reader;
    try {
      while (!this.done) {
        const { value, done } = await reader.read();
        if (done) {
          this.done = true;
          if (this.buffer.trim()) {
            const events = this.parseSSE(this.buffer);
            for (const event of events) {
              if (event !== null) {
                yield event;
              }
            }
          }
          break;
        }
        this.buffer += this.decoder.decode(value, { stream: true });
        const parts = this.buffer.split(/\n\n/);
        this.buffer = parts.pop() ?? "";
        for (const part of parts) {
          const events = this.parseSSE(part);
          for (const event of events) {
            if (event !== null) {
              yield event;
            }
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  }
  /**
   * Parse SSE format and extract data.
   * Returns null for [DONE] or empty events.
   *
   * SSE format:
   * - Lines starting with `:` are comments and are ignored
   * - `data:` lines contain the event payload
   * - Empty lines separate events
   * - We only process `data:` fields, ignoring `event:`, `id:`, `retry:`
   */
  parseSSE(text) {
    const results = [];
    const lines = text.split("\n");
    let hasDataLine = false;
    for (const line of lines) {
      if (!line) {
        continue;
      }
      if (line.startsWith(":")) {
        continue;
      }
      if (line.startsWith("data:")) {
        hasDataLine = true;
        const data = line.slice(5).trim();
        if (data === "[DONE]") {
          this.done = true;
          continue;
        }
        if (!data) {
          continue;
        }
        const parsed = JSON.parse(data);
        if (isResponseError(parsed)) {
          throw new ObjectiveAIFetchError(parsed);
        }
        results.push(parsed);
      }
    }
    if (!hasDataLine && results.length === 0) {
      return [];
    }
    return results;
  }
  /**
   * Collect all events into an array.
   */
  async toArray() {
    const results = [];
    for await (const item of this) {
      results.push(item);
    }
    return results;
  }
};

// src/viewer/apiCallBridge.ts
function buildCliArgs(method, path, body) {
  const segments = path.replace(/^\//, "").split("/").filter((s) => s.length > 0);
  const args = ["objectiveai", "api", ...segments, method.toLowerCase()];
  if (body !== null && body !== void 0) {
    args.push("--body-inline", JSON.stringify(body));
  }
  return args;
}
function viewerApiCallChunks(method, path, body) {
  const args = buildCliArgs(method, path, body);
  const cliOutput = invokeCli(args);
  return {
    [Symbol.asyncIterator]() {
      const inner = cliOutput[Symbol.asyncIterator]();
      return {
        async next() {
          for (; ; ) {
            const result = await inner.next();
            if (result.done) {
              return { value: void 0, done: true };
            }
            const line = result.value;
            if (!line || typeof line !== "object") continue;
            switch (line.type) {
              case "begin":
              case "end":
                continue;
              case "notification":
                return { value: line.value, done: false };
              case "error": {
                const message = typeof line.message === "string" ? line.message : JSON.stringify(line.message ?? line);
                throw new ObjectiveAIFetchError(0, message);
              }
              default:
                continue;
            }
          }
        },
        async return() {
          if (inner.return) {
            await inner.return();
          }
          return { value: void 0, done: true };
        }
      };
    }
  };
}
async function viewerApiCallUnary(method, path, body) {
  const iter = viewerApiCallChunks(method, path, body);
  for await (const chunk of iter) {
    return chunk;
  }
  throw new Error(
    `viewer api call ${method} ${path} produced no chunks before end`
  );
}
async function viewerApiCallUnaryNoResponse(method, path, body) {
  const iter = viewerApiCallChunks(method, path, body);
  for await (const _ of iter) {
  }
}
function viewerApiCallStream(method, path, body) {
  return Stream.fromAsyncIterable(viewerApiCallChunks(method, path, body));
}
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
}).describe("Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (built-ins: `agent_completions` /\n`functions_executions` / `functions_inventions_recursive` /\n`laboratories_executions`; plugins: whatever they declared in\ntheir manifest's `viewer_routes[i].type`).").meta({ "variantTitle": "Inbound" }), zod.z.object({
  destination: zod.z.string(),
  type: zod.z.literal("cli_command"),
  value: JsonValueSchema
}).describe("Host \u2192 iframe. One JSON line of cli output emitted by an\nin-process `objectiveai_cli::run()` invocation started by\nthis iframe via `invokeCli`. `value` is the cli's `Output<T>`\nenvelope. No sub_type \u2014 a single invocation produces a single\nstream of lines.").meta({ "variantTitle": "CliCommand" })]).describe("Every event the viewer emits to the JS side. Serde-tagged on\n`type` so the JS bridge can pattern-match and decide how to\nrepackage each variant for the destination iframe.\n\n`destination` is `\"objectiveai\"` for built-in events, or the\nplugin's repository name otherwise. For `CliCommand` it's the\nrepository name of whichever iframe invoked the CLI \u2014 the bridge\nderives it from `MessageEvent.source`, the plugin author never\nsets it.").meta({ title: "viewer.Event" });

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
function invokeCli(args) {
  return {
    [Symbol.asyncIterator]() {
      const queue = [];
      let resolveNext = null;
      let done = false;
      let cleaned = false;
      const onMessage = (event) => {
        const msg = event.data;
        if (!msg || typeof msg !== "object") return;
        if (msg.kind !== "plugin-event" || msg.type !== "cli_command") return;
        queue.push(msg.value);
        if (typeof msg.value === "object" && msg.value !== null && msg.value.type === "end") {
          done = true;
        }
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };
      const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        if (typeof window !== "undefined") {
          window.removeEventListener("message", onMessage);
        }
      };
      if (typeof window === "undefined" || window.parent === window) {
        console.warn(
          "@objectiveai/sdk/viewer: invokeCli() called outside an iframe; no host present, the iterator will yield nothing."
        );
        done = true;
      } else {
        window.addEventListener("message", onMessage);
        window.parent.postMessage({ kind: "cli-invoke", args }, "*");
      }
      return {
        async next() {
          while (queue.length === 0 && !done) {
            await new Promise((r) => {
              resolveNext = r;
            });
          }
          if (queue.length > 0) {
            return { value: queue.shift(), done: false };
          }
          cleanup();
          return { value: void 0, done: true };
        },
        async return() {
          cleanup();
          return { value: void 0, done: true };
        }
      };
    }
  };
}
function __resetForTests() {
  inboundListeners.clear();
}

exports.ViewerEventSchema = ViewerEventSchema;
exports.__resetForTests = __resetForTests;
exports.invokeCli = invokeCli;
exports.listen = listen2;
exports.viewerApiCallChunks = viewerApiCallChunks;
exports.viewerApiCallStream = viewerApiCallStream;
exports.viewerApiCallUnary = viewerApiCallUnary;
exports.viewerApiCallUnaryNoResponse = viewerApiCallUnaryNoResponse;
