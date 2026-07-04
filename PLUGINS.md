# ObjectiveAI Plugins

Plugins extend ObjectiveAI with a CLI binary, a viewer UI tab, or both. They live on GitHub, are installed via `objectiveai plugins install`, and are sandboxed at runtime — the host viewer iframes their UI with no DOM access to the parent, and the CLI dispatcher pipes JSON over stdin/stdout to their binary.

For component-specific details after reading this overview:

- **[CLI plugins reference](objectiveai-cli/PLUGINS.md)** — installing, listing, getting, and running plugins from the command line; the JSONL dispatch protocol; on-disk layout.
- **[Viewer plugins reference](objectiveai-viewer/PLUGINS.md)** — the GUI surface: tabs, the event envelope, the TypeScript SDK, the iframe + postMessage bridge.

## What a plugin is

A plugin is a GitHub repository that ships:

- A `objectiveai.json` **manifest** at the repo root, declaring a
  per-OS **`exec` command** the host runs for the plugin's CLI side.
- *Optionally* a **cli bundle** as a release asset (a zip extracted into
  the plugin's `cli/` folder — the exec working directory).
- *Optionally* a **viewer bundle** as a release asset (a zip with `index.html` + assets that renders inside the viewer's iframe sandbox).

The host viewer and CLI consume them like this:

```
GitHub repo                         Local installation
─────────                           ─────────────────────────────────────
objectiveai.json   ────install─►   <plugin_dir>/objectiveai.json   ← manifest (carries exec)
cli_zip            ────install─►   <plugin_dir>/cli/               ← exec CWD / payload
viewer_zip         ────install─►   <plugin_dir>/viewer/            ← viewer UI bundle

Then:
  CLI: `objectiveai plugins run …`            → runs exec[platform] with CWD=cli/, pipes JSONL
  Viewer: tab appears for plugins with        → iframe loads
          viewer_zip; routes register on        plugin://localhost/<repo>/index.html
          the embedded axum server
```

Plugins are **content-addressed by repository** — the name a plugin lives under on disk is whatever you pass to `--repository` at install time. The name `objectiveai` is reserved and cannot be used.

## The manifest

`objectiveai.json` at the repo root. Every field except `description` and `version` is optional.

```json
{
  "description":  "One-line summary surfaced in plugins list and discovery UIs.",
  "version":      "0.1.0",
  "author":       "Your Name",
  "homepage":     "https://github.com/you/repo",
  "license":      "MIT",

  "exec": {
    "windows": ["./my-plugin.exe"],
    "linux":   ["./my-plugin"],
    "macos":   ["./my-plugin"]
  },
  "cli_zip": "my-plugin-cli.zip",

  "viewer_zip": "my-plugin-viewer.zip",

  "mobile_ready": false
}
```

Field-by-field:

| Field | Type | Meaning |
|---|---|---|
| `description` | string | Required. One-line summary. |
| `version` | string | Required. Used to construct the release-asset URLs (`releases/download/v<version>/<asset>`). Semver recommended; not enforced. |
| `author` / `homepage` / `license` | string | Optional metadata. |
| `exec` | object | Per-OS argv (`windows` / `linux` / `macos`, each an array: program + leading args). The host appends the run's args and executes with CWD = the plugin's `cli/` folder; relative program paths resolve against it, bare names use PATH lookup. Omit for viewer-only plugins. |
| `cli_zip` | string | Asset filename for the cli bundle — a zip extracted into `cli/` at install time. Omit when the exec needs nothing on disk (e.g. it invokes a PATH-resolved program). |
| `viewer_zip` | string | Asset filename for the UI bundle. Omit if the plugin is CLI-only. |
| `mobile_ready` | bool | Opt-in flag for iOS/Android viewer builds. Future feature; default false. |

The canonical schema lives in [`objectiveai-rs/src/filesystem/plugins/manifest.rs`](objectiveai-rs/src/filesystem/plugins/manifest.rs).

## How to create a plugin

1. **Create a public GitHub repo** under your org or user. The repo name becomes the plugin's identifier on disk.

2. **Build a CLI program** that speaks the JSONL protocol on stdin/stdout, and declare how to launch it in the manifest's `exec`. See the [CLI plugins reference](objectiveai-cli/PLUGINS.md) for the wire format. The smallest possible compliant plugin lives at [`.objectiveai/bin/plugins/objectiveai/hello/0.0.1/cli/`](.objectiveai/bin/plugins/objectiveai/hello/0.0.1/cli/) (~10 lines of Rust, exec'd via `cargo run`).

3. **(Optional) Build a viewer bundle.** A static `dist/` from your favourite frontend stack (vite, plain HTML, anything). Use `ViewerPluginExecutor` (from `@objectiveai/sdk`) to run commands on the host; inbound host→plugin data delivery arrives later. See the [viewer plugins reference](objectiveai-viewer/PLUGINS.md).

4. **Write `objectiveai.json`** at your repo root with the fields above.

5. **Cut a GitHub release** tagged `v<version>` (matching the manifest's `version`). Upload the cli zip + the viewer zip as release assets named exactly as the manifest declares.

6. **Test the install**:
   ```bash
   objectiveai plugins install --owner <you> --repository <repo>
   ```
   This will fail until your repo is in the install whitelist (see [Distribution + trust](#distribution--trust) below). Use `--allow-untrusted` to bypass during development.

## Distribution + trust

Plugins are fetched from GitHub Releases. The host follows this URL convention:

- Manifest: `https://raw.githubusercontent.com/<owner>/<repo>/<commit-or-HEAD>/objectiveai.json`
- Cli-zip / viewer-zip: `https://github.com/<owner>/<repo>/releases/download/v<version>/<asset-name>`

The CLI ships with an install **whitelist**. By default, only repos under the `ObjectiveAI` GitHub org (case-insensitive) are installable; everything else requires the `--allow-untrusted` flag, which emits a `warn`-level notification at install time documenting that the plugin is unverified.

The whitelist lives in [`objectiveai-rs/src/filesystem/plugins/whitelist.rs`](objectiveai-rs/src/filesystem/plugins/whitelist.rs). Each entry is `{ owner, repository, commit_sha, version }` — each field a regex (anchored). The default is one entry: `owner = "(?i)ObjectiveAI"`, all others `".*"`.

## Installing a plugin

```bash
# From the ObjectiveAI org — works by default.
objectiveai plugins install --owner ObjectiveAI --repository my-plugin

# From anywhere else — requires the explicit opt-in.
objectiveai plugins install --owner third-party --repository thing --allow-untrusted

# Pin to a specific commit (otherwise the default branch is used).
objectiveai plugins install --owner ObjectiveAI --repository my-plugin --commit-sha <sha>

# Replace an already-installed plugin (cli/ + viewer/ + manifest are
# rewritten; extra runtime data under <repo>/ is preserved).
objectiveai plugins install --owner ObjectiveAI --repository my-plugin --upgrade
```

For the full set of flags and behaviour (errors, transactional semantics, what gets written where), see the [CLI plugins reference](objectiveai-cli/PLUGINS.md).

## Where to go next

- **[CLI plugins reference](objectiveai-cli/PLUGINS.md)** — installing, listing, getting, running plugins; the JSONL dispatch protocol; on-disk layout.
- **[Viewer plugins reference](objectiveai-viewer/PLUGINS.md)** — building a UI tab: viewer bundle, the event envelope, the SDK, the iframe + postMessage bridge.
