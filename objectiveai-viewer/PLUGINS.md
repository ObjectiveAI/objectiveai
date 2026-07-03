# Viewer Plugins Reference

Start with the [root plugin overview](../PLUGINS.md) for the model and the manifest schema. For CLI dispatch and installation behaviour, see the [CLI plugins reference](../objectiveai-cli/PLUGINS.md). This doc is the GUI side: how a plugin shows up as a tab, the wire format the viewer emits to it, and the TypeScript SDK plugin authors use inside their iframe.

## What a viewer plugin is

When the viewer starts up, it reads the persisted manifests in `<plugins_dir>/*.json` and surfaces one tab per plugin that ships a `viewer_zip`. The leftmost tab is always `ObjectiveAI` (the root view); plugin tabs follow.

Clicking a plugin tab mounts a sandboxed `<iframe>` pointed at the plugin's UI bundle, served by the host through a custom `plugin://` URI scheme. Each iframe is allow-listed to talk back to the host via a postMessage bridge.

The viewer itself is a WebSocket client of the CLI daemon's broadcast stream — it is not a server and exposes no HTTP endpoints. Plugin iframes receive data when a `plugins/run` targeting their plugin executes anywhere on the machine: the run's request and stream items are routed into the plugin's tab as events.

## The viewer bundle

A zip whose **root** contains `index.html` plus whatever assets it references. Extracted by `install_plugin` into `<plugins_dir>/<repository>/viewer/`.

The host serves it via a custom Tauri URI scheme: `plugin://localhost/<repository>/<path>`. The iframe's `src` is set to `plugin://localhost/<repository>/index.html`. Sandbox:

```html
<iframe sandbox="allow-scripts allow-forms" />
```

No `allow-same-origin` — the iframe has its own origin, can't read parent globals, can't share cookies/localStorage with the host. All host interaction is via `window.parent.postMessage`.

Path-traversal defense: any `..` segment in a request to `plugin://` is rejected outright. The handler canonicalizes the path and checks it remains inside `<plugins_dir>/<repository>/viewer/`.

MIME types are guessed by `mime_guess::from_path(...).first_or_octet_stream()`. No charset is appended — include `<meta charset="utf-8">` in your `index.html`.

## Receiving data: the daemon stream

The host connects to the CLI daemon's broadcast WebSocket and watches every CLI run on the machine. When a `plugins/run` request targets your plugin and your tab is open, the run is forwarded into your iframe as `plugins_run` events:

- First the **request frame**: `{…context, id, value: <the plugins/run Request>}`.
- Then one **response frame** per stream item: `{id, path_type: "plugins/run", value: <item>}`.

The `id` is the same across a run's frames, so a plugin can follow multiple concurrent runs. Frames for plugins without an open tab are dropped — plugins observe live activity, they don't replay history.

## The event envelope

Every Tauri event the viewer emits — built-in or plugin — uses this shape:

```json
{
  "destination": "<repo or 'objectiveai'>",
  "type":        "<snake_case kind>",
  "sub_type":    "<discriminator, inbound events only>",
  "value":       <original payload>
}
```

The Tauri **channel name equals `destination`**. Built-in events fire on channel `"objectiveai"`; plugin events fire on channel `<repository>`. The repository name `objectiveai` is reserved at install time so plugin channels can never collide with built-in events.

`type` is `inbound` (host has data for the listener; `sub_type` discriminates further — the raw daemon firehose arrives on the `objectiveai` channel as `sub_type: "daemon"`, and routed run frames reach plugin iframes as `sub_type: "plugins_run"`) or `cli_command` (one stdout JSONL line from a CLI invocation this iframe started).

The Rust definition lives in [`objectiveai-sdk-rs/src/viewer/events.rs`](../objectiveai-sdk-rs/src/viewer/events.rs).

## The TypeScript SDK

Plugin UIs use [`@objectiveai/viewer-sdk`](../objectiveai-viewer-sdk/) (workspace package; npm publication pending). One function:

```ts
import { listen } from "@objectiveai/viewer-sdk";

// Listen for events the host emits to your plugin. The first argument
// matches the incoming event's `sub_type` — `"plugins_run"` for
// routed daemon run frames.
const unlisten = listen("plugins_run", (frame) => {
  // frame is the raw daemon frame: a request frame
  // ({…context, id, value}) or a response frame
  // ({id, path_type, value}).
  console.log("run frame:", frame);
});

// Later, when the iframe is being torn down:
unlisten();
```

The SDK detects context:

- **Production** (inside the host viewer's iframe): `window.parent !== window`, so the SDK subscribes to `postMessage` from `window.parent` and dispatches matching `plugin-event` messages to your handler.
- **Dev** (your plugin running in its own standalone Tauri shell): `window.parent === window`, so the SDK falls through to `@tauri-apps/api`'s `listen`. You get the same surface; only the transport differs.

Plugin authors can develop their plugin as a normal Tauri app locally (their `src-tauri` backend can emit events themselves for development), then ship the built `dist/` as `<repo>-viewer.zip`. The SDK shim is the only thing they need to use to make their plugin work in both contexts.

## The postMessage bridge

```
plugin iframe                    host React shell                Rust backend
─────────────                    ────────────────                ────────────
listen("plugins_run", h)
        ▲
        │ postMessage                                            Tauri emit
        │ {kind:"plugin-event",   forward to matching iframe ◄── channel=destination
        │  type, sub_type, value} per-plugin tauriListen          payload=Event
```

The bridge lives in [`objectiveai-viewer/src/plugin-bridge.ts`](src/plugin-bridge.ts). Its responsibilities:

- Track which iframe corresponds to which plugin (`registerIframe` / `unregisterIframe`).
- Subscribe per-plugin to the `<repository>` Tauri channel; forward each event to that plugin's iframe (and only that iframe — cross-plugin event leakage isn't possible).
- Watch the `objectiveai` channel's daemon stream and route `plugins/run` frames to the matching plugin's tab as `plugins_run` events.
- Carry the reverse direction: iframes post `cli-execute` (a typed `cli::command::Request`) and `api-call-invoke` messages; the bridge resolves the originating iframe via `MessageEvent.source` and dispatches the matching Tauri command with that plugin's name as `origin`. Messages from unknown windows are dropped.

## `mobile_ready`

Plugins that want to surface on the viewer's eventual iOS/Android builds set `"mobile_ready": true` in their manifest. The mobile viewer (Tauri 2 mobile, not yet built) will surface only `mobile_ready` plugins as tabs, because mobile has no local CLI binary — plugins must either be pure-frontend or talk to a remote backend over `fetch`.

Out of scope today; documented so the manifest schema doesn't need a future migration.

## Smallest compliant plugin

The CLI fixture at [`objectiveai-cli/test-fixtures/hello-plugin/`](../objectiveai-cli/test-fixtures/hello-plugin/) is the minimal binary-side example. A matching viewer-bundle fixture is pending; for now the smallest viewer bundle is just an `index.html` with:

```html
<!doctype html>
<title>my-plugin</title>
<p>Waiting for events…</p>
<script type="module">
  // The host posts `{kind: "plugin-event", type, sub_type, value}`
  // messages into this iframe whenever a plugins/run targeting this
  // plugin executes. @objectiveai/viewer-sdk is a thin wrapper around
  // this protocol; you can also use postMessage directly:
  window.addEventListener("message", (e) => {
    if (e.data?.kind !== "plugin-event") return;
    if (e.data.sub_type === "plugins_run") {
      document.querySelector("p").textContent =
        `Got: ${JSON.stringify(e.data.value)}`;
    }
  });
</script>
```

Zip it as `<repo>-viewer.zip`, set `viewer_zip` in your manifest, install. The viewer's tab will render this HTML and react whenever `objectiveai plugins run` targets this plugin.
