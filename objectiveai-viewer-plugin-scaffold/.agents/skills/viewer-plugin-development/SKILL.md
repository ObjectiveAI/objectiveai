---
name: viewer-plugin-development
description: Develop an ObjectiveAI viewer plugin against a running viewer — register the plugin root so the viewer serves tabs/scripts straight from the working tree with hot reload, paired with the MCP half under one identity. Use when writing or debugging viewer tabs, channel handlers, or browser scripts.
---

# Developing the viewer half

A viewer plugin only exists inside the ObjectiveAI viewer. Development
mode points the plugin's coordinates at the working tree so the viewer
serves `plugin://` straight from the built output — no container, no
install — and hot-reloads open tabs.

## Which directory to register

**The one holding `objectiveai.json` — the PLUGIN ROOT**, exactly as
for the MCP half. Two layouts:

- this half standing alone → the root is this directory, and
  `viewer.development.output` is `dist`;
- a full plugin (`mcp/` + `viewer/` under one root, what `scaffold.sh`
  produces) → the root is the PARENT, `viewer.development.output` is
  `viewer/dist`, and the watch build still runs inside `viewer/`.

`development.output` is a HOST path resolved against the REGISTERED
directory, which is why it changes between the two; `viewer.output`
(`/dist`) is a path inside the built image and never changes.

## The loop

1. Start the watch build in THIS half: `pnpm install` once, then
   `pnpm run dev` (writes the manifest's `development.output`).
2. Register (coordinates are yours; the version is a v-tag shape):

   ```
   objectiveai development plugins viewer create \
     --owner <owner> --name <name> --version v0.1.0 --path <ABSOLUTE path to the plugin root>
   ```

   This RESPAWNS a running viewer (registrations are frozen per viewer
   process). The dev plugin REPLACES any installed plugin with the same
   owner/name; install/uninstall of it are refused while registered.
3. Edit. CSS-only changes swap stylesheets in place; an entry-module
   change remounts the component; anything else reloads the webview. A
   changed SCRIPT closes every browser tab it was injected into —
   respawn the browser from your tab.
4. `objectiveai development plugins viewer delete --owner … --name …
   --version …` to go back to installed.

## One plugin, two halves

The MCP half registers with `development plugins mcp create` under the
SAME owner/name/version AND the same `--path` — the trios must match
BYTE FOR BYTE or channel offers will not route to this repo's handler.
Release = one repo carrying both halves under one `objectiveai.json`,
tagged `v<semver>`.

## What will bite you

- **React must stay external** in tab bundles (see build.mjs); a bundled
  React dies on the first hook with a confusing error.
- Every module/style the manifest declares must EXIST in the built
  output — missing files fail the release build and 404 in dev. The
  `module`/`styles` paths are relative to that output, so they do not
  change between the two layouts.
- Browser scripts have ONLY `__objectiveai.send/subscribe/list` (the
  child-side mailbox to the spawning tab). No Tauri, no SDK, no network
  back to the viewer. Treat everything a script sends as untrusted —
  it originates in a foreign page's JS world.
- A channel handler's props are
  `{ arguments: { request: <offer>, response: { secret } } }`; reply
  with `channelsLogsReplyExecute(Client.viewer(transport), { channel_id,
  secret, content })`.
