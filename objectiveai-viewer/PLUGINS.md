# Viewer Plugins Reference

Start with the [root plugin overview](../PLUGINS.md) for the model and the manifest schema. For CLI dispatch and installation behaviour, see the [CLI plugins reference](../objectiveai-cli/PLUGINS.md). This doc is the GUI side: how a plugin shows up as a tab, the wire format the viewer emits to it, and the TypeScript SDK plugin authors use inside their iframe.

## What a viewer plugin is

When the viewer starts up, it reads the persisted manifests in `<plugins_dir>/*.json` and surfaces one tab per plugin that ships a `viewer_zip`. The leftmost tab is always `ObjectiveAI` (the root view); plugin tabs follow.

Clicking a plugin tab mounts a sandboxed `<iframe>` pointed at the plugin's UI bundle, served by the host through a custom `plugin://` URI scheme. Each iframe is allow-listed to talk back to the host via a postMessage bridge.

A plugin can also (or alternatively) declare `viewer_routes` — HTTP endpoints the host's embedded axum server registers on the plugin's behalf. Anything that POSTs to one of those routes triggers an event to the plugin's iframe.

## The viewer bundle

A zip whose **root** contains `index.html` plus whatever assets it references. Extracted by `install_plugin` into `<plugins_dir>/<repository>/viewer/`.

The host serves it via a custom Tauri URI scheme: `plugin://localhost/<repository>/<path>`. The iframe's `src` is set to `plugin://localhost/<repository>/index.html`. Sandbox:

```html
<iframe sandbox="allow-scripts allow-forms" />
```

No `allow-same-origin` — the iframe has its own origin, can't read parent globals, can't share cookies/localStorage with the host. All host interaction is via `window.parent.postMessage`.

Path-traversal defense: any `..` segment in a request to `plugin://` is rejected outright. The handler canonicalizes the path and checks it remains inside `<plugins_dir>/<repository>/viewer/`.

MIME types are guessed by `mime_guess::from_path(...).first_or_octet_stream()`. No charset is appended — include `<meta charset="utf-8">` in your `index.html`.

## viewer_routes

Declare HTTP routes the host registers on your behalf:

```json
{
  "viewer_routes": [
    { "path": "/echo",   "method": "POST", "type": "echo_request" },
    { "path": "/status", "method": "GET",  "type": "status_request" }
  ]
}
```

| Field | Meaning |
|---|---|
| `path` | Must start with `/`. The host prepends `/plugin/<repository>` before registering. So your `/echo` lands at `/plugin/<your-repo>/echo` on the viewer's axum server. |
| `method` | `GET` / `POST` / `PUT` / `PATCH` / `DELETE`. |
| `type` | Free-form string. Forwarded to your iframe as the event's `type` field so you can switch on it. |

A hit on a plugin route doesn't return data to the HTTP caller (returns 200 OK). Instead it emits an event the plugin's iframe listens for.

## The event envelope

Every Tauri event the viewer emits — built-in or plugin — uses this shape:

```json
{
  "destination": "<repo or 'objectiveai'>",
  "type":        "<snake_case kind>",
  "value":       <original payload>
}
```

The Tauri **channel name equals `destination`**. Built-in events fire on channel `"objectiveai"`; plugin events fire on channel `<repository>`. The repository name `objectiveai` is reserved at install time so plugin channels can never collide with built-in events.

Built-in `type` values: `agent_completions`, `functions_executions`, `functions_inventions_recursive`, `laboratories_executions`. Their `value` is the raw POST body the matching axum route received.

Plugin `type` values: whatever you declared in `viewer_routes[i].type`. Plugin `value` is the JSON body of the HTTP request (or `null` for body-less GETs).

The Rust definition lives in [`objectiveai-viewer/src-tauri/src/events.rs`](src-tauri/src/events.rs).

## The TypeScript SDK

Plugin UIs use [`@objectiveai/viewer-sdk`](../objectiveai-viewer-sdk/) (workspace package; npm publication pending). One function:

```ts
import { listen } from "@objectiveai/viewer-sdk";

// Listen for events the host emits to your plugin. `type` matches the
// `type` field of an incoming event (the manifest-declared value of your
// viewer_routes entry).
const unlisten = listen<{ to: string }>("echo_request", (value) => {
  // value is event.value — the raw POST body
  console.log("got request:", value.to);
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
listen("echo_request", h)
        ▲
        │ postMessage                                            Tauri emit
        │ {kind:"plugin-event",   forward to matching iframe ◄── channel=destination
        │  type, value}           per-plugin tauriListen          payload=Event
```

The bridge lives in [`objectiveai-viewer/src/plugin-bridge.ts`](src/plugin-bridge.ts). Its responsibilities:

- Track which iframe corresponds to which plugin (`registerIframe` / `unregisterIframe`).
- Subscribe per-plugin to the `<repository>` Tauri channel; forward each event to that plugin's iframe (and only that iframe — cross-plugin event leakage isn't possible).

The bridge is event-forwarding only. There is no inbound channel: iframes don't call into the host. If a plugin needs to talk to a remote backend it does so directly via `fetch`.

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
  // into this iframe whenever one of our manifest-declared
  // viewer_routes is hit. @objectiveai/viewer-sdk is a thin wrapper
  // around this protocol; you can also use postMessage directly:
  window.addEventListener("message", (e) => {
    if (e.data?.kind !== "plugin-event") return;
    if (e.data.type === "echo_request") {
      document.querySelector("p").textContent =
        `Got: ${JSON.stringify(e.data.value)}`;
    }
  });
</script>
```

Zip it as `<repo>-viewer.zip`, set `viewer_zip` in your manifest, install. The viewer's tab will render this HTML and react to POSTs at `/plugin/<repo>/echo`.
