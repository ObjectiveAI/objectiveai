import { z } from 'zod';

// src/viewer/destination.ts
var ViewerDestinationSchema = z.union([z.literal("objectiveai").describe("The main viewer UI.").meta({ "variantTitle": "Objectiveai" }), z.object({
  plugin: z.object({
    name: z.string(),
    owner: z.string(),
    version: z.string()
  })
}).strict().describe("One plugin's iframe.").meta({ "variantTitle": "Plugin" })]).describe("Where an event is delivered: the main ObjectiveAI viewer UI, or one\nplugin's iframe (identified by its full install coordinates).").meta({ title: "viewer.Destination" });
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
  destination: ViewerDestinationSchema,
  type: z.literal("inbound"),
  value: JsonValueSchema
}).describe("Host \u2192 JS. Data for the destination \u2014 today that's the daemon\n`/listen` passthrough (standard broadcast envelope frames),\ndestined to the main viewer UI. Nothing is emitted to plugin\ndestinations on this variant yet.").meta({ "variantTitle": "Inbound" }), z.object({
  destination: ViewerDestinationSchema,
  id: z.string(),
  type: z.literal("cli_command"),
  value: JsonValueSchema
}).describe('Host \u2192 JS. One response line from a viewer-executor invocation\nthe destination itself started, terminated by a synthetic\n`{"type":"end"}` line \u2014 whoever runs a request gets its own\nresponse back, main UI and plugins alike. `id` is the\ninvocation id the CALLER minted when it posted the request:\nit rides every response line, so concurrent invocations from\none destination demux cleanly.').meta({ "variantTitle": "CliCommand" })]).describe("Every event the viewer emits to the JS side. Serde-tagged on\n`type` so the JS side can pattern-match each variant.").meta({ title: "viewer.Event" });

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
        const mod = await import('./event-T244TCB6.js');
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

export { JsonValueSchema, ViewerDestinationSchema, ViewerEventSchema, __resetForTests, listen };
