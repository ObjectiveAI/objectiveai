# Viewer Plugins Reference

Start with the [root plugin overview](../PLUGINS.md) for the model and the manifest schema. For CLI dispatch and installation behaviour, see the [CLI plugins reference](../objectiveai-cli/PLUGINS.md). This doc is the GUI side: how a plugin shows up as a tab, the wire format the viewer emits to it, and the TypeScript SDK plugin authors use inside their iframe.

## What a viewer plugin is

When the viewer starts up, it reads the persisted manifests in `<plugins_dir>/*.json` and surfaces one tab per plugin that ships a `viewer_zip`. The leftmost tab is always `ObjectiveAI` (the root view); plugin tabs follow.

Clicking a plugin tab mounts a sandboxed `<iframe>` pointed at the plugin's UI bundle, served by the host through a custom `plugin://` URI scheme. Each iframe is allow-listed to talk back to the host via a postMessage bridge.

The viewer itself is a WebSocket client of the CLI daemon's broadcast stream — it is not a server and exposes no HTTP endpoints. For now, the only events delivered into a plugin's iframe are the responses to requests the plugin itself runs through the viewer executor (`plugins/run` delivery to plugin tabs comes later).

## The viewer bundle

A zip whose **root** contains `index.html` plus whatever assets it references. Extracted by `install_plugin` into `<plugins_dir>/<repository>/viewer/`.

The host serves it via a custom Tauri URI scheme: `plugin://localhost/<repository>/<path>`. The iframe's `src` is set to `plugin://localhost/<repository>/index.html`. Sandbox:

```html
<iframe sandbox="allow-scripts allow-forms" />
```

No `allow-same-origin` — the iframe has its own origin, can't read parent globals, can't share cookies/localStorage with the host. All host interaction is via `window.parent.postMessage`.

Path-traversal defense: any `..` segment in a request to `plugin://` is rejected outright. The handler canonicalizes the path and checks it remains inside `<plugins_dir>/<repository>/viewer/`.

MIME types are guessed by `mime_guess::from_path(...).first_or_octet_stream()`. No charset is appended — include `<meta charset="utf-8">` in your `index.html`.

## The event envelope

Every Tauri event the viewer emits uses this shape:

```json
{
  "type":        "<inbound | cli_command>",
  "destination": "objectiveai" | { "plugin": { "owner", "name", "version" } },
  "value":       <payload>
}
```

Events fan out on exactly two Tauri channels, keyed by `destination`: `"objectiveai"` (the main viewer UI — this includes the daemon `/listen` passthrough frames) and the shared `"plugin"` channel, where `destination.plugin` carries the target plugin's full install coordinates. The JS side does no routing beyond delivering plugin-destined events to the matching iframe.

`type` is `cli_command` (one response line from a viewer-executor invocation the destination itself started, terminated by a synthetic `{"type":"end"}` line — whoever runs a request gets its own response back, main UI and plugins alike) or `inbound` (host data for the destination; nothing is emitted to plugin destinations on it yet).

The Rust definition lives in [`objectiveai-sdk-rs/src/viewer/events.rs`](../objectiveai-sdk-rs/src/viewer/events.rs).

## The TypeScript SDK

Plugin UIs use [`@objectiveai/viewer-sdk`](../objectiveai-viewer-sdk/) (workspace package; npm publication pending). To run commands, use the `ViewerCommandExecutor` (from `@objectiveai/sdk`) with the generated execute functions — it posts `cli-execute` messages to the host bridge and consumes the `cli_command` responses streamed back to this plugin. Inbound host→plugin data delivery (e.g. `plugins/run` runs) is not emitted yet; the `RunListener` in `@objectiveai/sdk/viewer` is the receiving surface it will arrive on.

The SDK detects context:

- **Production** (inside the host viewer's iframe): `window.parent !== window`, so the SDK subscribes to `postMessage` from `window.parent` and dispatches matching `plugin-event` messages to your handler.
- **Dev** (your plugin running in its own standalone Tauri shell): `window.parent === window`, so the SDK falls through to `@tauri-apps/api`'s `listen`. You get the same surface; only the transport differs.

Plugin authors can develop their plugin as a normal Tauri app locally (their `src-tauri` backend can emit events themselves for development), then ship the built `dist/` as `<repo>-viewer.zip`. The SDK shim is the only thing they need to use to make their plugin work in both contexts.

## The postMessage bridge

```
plugin iframe                    host React shell                Rust backend
─────────────                    ────────────────                ────────────
invokeCli(request) ⇄ responses
        ▲
        │ postMessage                                            Tauri emit
        │ {kind:"plugin-event",   deliver to destination's  ◄── channel="plugin"
        │  type, value}           iframe (full coordinates)      payload=Event
```

The bridge lives in [`objectiveai-viewer/src/plugin-bridge.ts`](src/plugin-bridge.ts). Its responsibilities:

- Track which iframe corresponds to which plugin coordinates (`registerIframe` / `unregisterIframe`).
- Subscribe once to the shared `"plugin"` Tauri channel and deliver each event to the destination's iframe (and only that iframe — cross-plugin event leakage isn't possible).
- Carry the reverse direction: iframes post `cli-execute` (a typed `cli::command::Request`) messages; the bridge resolves the originating iframe via `MessageEvent.source` and dispatches the `cli_execute` Tauri command with that plugin's full coordinates as the `destination` — a plugin never claims an identity itself. Messages from unknown windows are dropped.

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
  // The host posts `{kind: "plugin-event", type, value}` messages
  // into this iframe — today that's the `cli_command` responses to
  // requests this plugin runs through the viewer executor.
  // @objectiveai/viewer-sdk wraps this protocol; you can also use
  // postMessage directly:
  window.addEventListener("message", (e) => {
    if (e.data?.kind !== "plugin-event") return;
    document.querySelector("p").textContent =
      `Got: ${JSON.stringify(e.data.value)}`;
  });
</script>
```

Zip it as `<repo>-viewer.zip`, set `viewer_zip` in your manifest, install. The viewer's tab will render this HTML and react whenever `objectiveai plugins run` targets this plugin.
