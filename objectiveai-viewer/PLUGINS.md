# Viewer Plugins Reference

Start with the [root plugin overview](../PLUGINS.md) for the model and the manifest schema. For CLI dispatch and installation behaviour, see the [CLI plugins reference](../objectiveai-cli/PLUGINS.md). This doc is the GUI side: how a plugin shows up as a tab, the wire format the viewer emits to it, and the TypeScript SDK plugin authors use inside their iframe.

## What a viewer plugin is

When the viewer starts up, it reads the persisted manifests in `<plugins_dir>/*.json` and surfaces one tab per plugin that ships a `viewer_zip`. The leftmost tab is always `ObjectiveAI` (the root view); plugin tabs follow.

Clicking a plugin tab mounts a sandboxed `<iframe>` pointed at the plugin's UI bundle, served by the host through a custom `plugin://` URI scheme. Each iframe is allow-listed to talk back to the host via a postMessage bridge.

The viewer's data path is JS-native: the main viewer's frontend connects to the CLI daemon directly over native WebSockets (`/listen` and `/execute`); the Rust side holds no daemon stream and only hands the frontend the daemon address + auth signature via the `daemon_config` Tauri command. Plugins never see those credentials — their only channel is the postMessage bridge with the host window. For now, the only events delivered into a plugin's iframe are the responses to requests the plugin itself runs through the plugin executor (`plugins/run` delivery to plugin tabs comes later).

## The viewer bundle

A zip whose **root** contains `index.html` plus whatever assets it references. Extracted by `install_plugin` into `<plugins_dir>/<repository>/viewer/`.

The host serves it via a custom Tauri URI scheme: `plugin://localhost/<repository>/<path>`. The iframe's `src` is set to `plugin://localhost/<repository>/index.html`. Sandbox:

```html
<iframe sandbox="allow-scripts allow-forms" />
```

No `allow-same-origin` — the iframe has its own origin, can't read parent globals, can't share cookies/localStorage with the host. All host interaction is via `window.parent.postMessage`.

Path-traversal defense: any `..` segment in a request to `plugin://` is rejected outright. The handler canonicalizes the path and checks it remains inside `<plugins_dir>/<repository>/viewer/`.

MIME types are guessed by `mime_guess::from_path(...).first_or_octet_stream()`. No charset is appended — include `<meta charset="utf-8">` in your `index.html`.

## The plugin-event envelope

Every message the host posts into a plugin's iframe uses this shape:

```json
{
  "kind":  "plugin-event",
  "type":  "<cli_command | inbound>",
  "id":    "<invocation id — cli_command only>",
  "value": <payload>
}
```

`type` is `cli_command` (one response line from a plugin-executor invocation THIS plugin started, carrying the invocation `id` the plugin minted — concurrent invocations demux by it — and terminated by a synthetic `{"type":"end"}` line) or `inbound` (host data for the plugin; nothing is emitted on it yet — `plugins/run` delivery comes later).

## The TypeScript SDK

Plugin UIs use [`@objectiveai/viewer-sdk`](../objectiveai-viewer-sdk/) (workspace package; npm publication pending). To run commands, use the `ViewerPluginExecutor` (from `@objectiveai/sdk`) with the generated execute functions — it posts `cli-execute` messages to the host bridge and consumes the `cli_command` responses streamed back to this plugin. It is plugin-only and throws outside an iframe; in the main viewer, use `WebSocketExecutor` (the daemon is directly reachable there). Inbound host→plugin data delivery (e.g. `plugins/run` runs) is not emitted yet; the receiving surface will arrive with it. (The `WebSocketListener` in `@objectiveai/sdk` is NOT that surface — it consumes the daemon broadcast directly and plugins have no daemon credentials.)

The SDK detects context:

- **Production** (inside the host viewer's iframe): `window.parent !== window`, so the SDK subscribes to `postMessage` from `window.parent` and dispatches matching `plugin-event` messages to your handler.
- **Dev** (your plugin running in its own standalone Tauri shell): `window.parent === window`, so the SDK falls through to `@tauri-apps/api`'s `listen`. You get the same surface; only the transport differs.

Plugin authors can develop their plugin as a normal Tauri app locally (their `src-tauri` backend can emit events themselves for development), then ship the built `dist/` as `<repo>-viewer.zip`. The SDK shim is the only thing they need to use to make their plugin work in both contexts.

## The postMessage bridge

```
plugin iframe                    host React shell                 CLI daemon
─────────────                    ────────────────                 ──────────
executor.execute(request)
        │ postMessage
        │ {kind:"cli-execute",    resolve iframe identity,   ──►  /execute
        │  id, request}           run via JS WebSocketExecutor    (native WS)
        ▲
        │ postMessage
        │ {kind:"plugin-event",   every response line + the  ◄──  JSONL lines
        │  type:"cli_command",    synthetic {"type":"end"}
        │  id, value}             marker, back to THAT iframe
```

The bridge lives in [`objectiveai-viewer/src/plugin-bridge.ts`](src/plugin-bridge.ts). Its responsibilities:

- Track which iframe corresponds to which plugin coordinates (`registerIframe` / `unregisterIframe`).
- Carry executions: iframes post `cli-execute` (a typed `cli::command::Request`) messages with a self-minted invocation `id`; the bridge resolves the originating iframe via `MessageEvent.source` (a plugin never claims an identity itself; unknown windows and id-less messages are dropped), runs the request through the host's own JS-native `WebSocketExecutor` — daemon address + signature from the Rust `daemon_config` command, fetched once — and posts every line + the end marker back into that iframe only. Failures surface as one in-band `{"type":"error",…}` line, then the end marker.

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
  // The host posts `{kind: "plugin-event", type, id, value}`
  // messages into this iframe — today that's the `cli_command`
  // responses to requests this plugin runs through the plugin
  // executor.
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
