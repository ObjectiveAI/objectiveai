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
function listen(sub_type, handler) {
  if (!isInIframe()) {
    let unlisten = null;
    let cancelled = false;
    void (async () => {
      try {
        const mod = await import('./event-MA6ITXCE.js');
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

export { __resetForTests, invokeCli, listen };
