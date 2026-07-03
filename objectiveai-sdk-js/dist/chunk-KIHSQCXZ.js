import { z } from 'zod';

// src/viewer/event.ts
var JsonValueSchema = z.union([
  z.string(),
  z.number(),
  z.boolean(),
  z.null(),
  z.array(z.lazy(() => JsonValueSchema)),
  z.record(z.string(), z.lazy(() => JsonValueSchema))
]);

// src/viewer/event.ts
var ViewerEventSchema = z.union([z.object({
  destination: z.string(),
  sub_type: z.string(),
  type: z.literal("inbound"),
  value: JsonValueSchema
}).describe('Host \u2192 iframe. Carries data into the plugin (the existing\npath). `sub_type` is the snake_case discriminator the plugin\nlistens on (e.g. `daemon` for raw daemon-broadcast frames on\nthe `"objectiveai"` channel; the JS bridge repackages routed\n`plugins/run` frames as `plugins_run` for plugin iframes).').meta({ "variantTitle": "Inbound" }), z.object({
  destination: z.string(),
  type: z.literal("cli_command"),
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
function __resetForTests() {
  inboundListeners.clear();
}

export { JsonValueSchema, ViewerEventSchema, __resetForTests, listen };
