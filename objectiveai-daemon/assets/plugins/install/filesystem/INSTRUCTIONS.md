# Authoring a Local Plugin

You're authoring a plugin under `~/.objectiveai/plugins/` by hand. The
CLI does not install anything on this path — it only hands you these
instructions. Follow the steps in order.

## 1. Fetch the manifest schema

Don't guess the manifest fields. The CLI ships the live JSON Schema:

    objectiveai schemas filesystem plugins Manifest get

Read all of it before writing anything to disk.

## 2. Write the manifest

Choose a plugin name. The name is both the directory under
`~/.objectiveai/plugins/` and the sidecar JSON filename. Use lowercase
ASCII letters, digits, `.`, `_`, or `-` — the same character set
GitHub allows for repository names.

Create the manifest at:

    ~/.objectiveai/plugins/<name>.json

Required fields are `description` and `version` (semver-style string,
e.g. `0.1.0`). Everything else is optional. **`binaries` is for
GitHub-install plugins only** — local plugins ship the binary directly
in the plugin directory (next step), so leave `binaries` out.

## 3. Place the plugin binary

Drop the binary at:

    ~/.objectiveai/plugins/<name>/<filename>

`<filename>` can be any of:

- `plugin` (canonical on Linux / macOS)
- `plugin.exe` (canonical on Windows)
- `plugin.<any-extension>` (e.g. `plugin.sh`, `plugin.py`,
  `plugin.bat`, `plugin.js`) — the CLI's plugin resolver scans for any
  file with stem `plugin` and falls back through three tiers. On
  Windows: `plugin.exe` → `plugin` → first matching `plugin.*`. On
  other platforms: `plugin` → `plugin.exe` → first matching `plugin.*`.

The file can be a compiled binary OR a script. The CLI calls
`Command::new(path)`, and the OS picks the right runner via shebang
(Unix) or extension association (Windows). For a shell script with a
shebang, set the executable bit on Unix:

    chmod +x ~/.objectiveai/plugins/<name>/plugin

## 4. Set up the viewer (optional)

If your plugin has a UI tab in the viewer, pick ONE of the two
sources. They're mutually exclusive — setting both makes the manifest
invalid.

### Option A — `viewer_url` (recommended for development)

Run your viewer dev server (Vite, Next.js, whatever) on a local port,
e.g. `http://localhost:5173`. Set the URL in the manifest:

    "viewer_url": "http://localhost:5173"

The viewer loads it directly into the iframe — your dev server's hot
reload propagates straight in. Edit-save-see-update with no plugin
reinstall. This is the loop you want during development.

Allowed schemes: `https://`, `http://localhost*`, `http://127.0.0.1*`.
Anything else (raw `http://` on a public hostname, `ftp://`, …) is
rejected at manifest-parse time.

### Option B — bundled `viewer/` directory

For a production-ish local plugin without a dev server, place
pre-built static assets (an `index.html` plus JS / CSS / images)
directly under:

    ~/.objectiveai/plugins/<name>/viewer/

Leave `viewer_url` AND `viewer_zip` unset in the manifest. The
viewer's `plugin://` URI handler serves files out of that directory;
the iframe loads `plugin://localhost/<name>/index.html`. No hot
reload — rebuild + restart the viewer to see changes.

### Receiving data

The viewer feeds plugin iframes from the CLI daemon's broadcast
stream: when a `plugins/run` targeting your plugin executes anywhere
on the machine, the run's request and stream items arrive in your
iframe as `inbound` events with `sub_type: "plugins_run"`. Subscribe
via `listen("plugins_run", handler)` from `@objectiveai/sdk`.

## 5. Restart the viewer

The viewer scans `~/.objectiveai/plugins/` on startup. New plugins
and manifest changes are picked up at restart, not live.
