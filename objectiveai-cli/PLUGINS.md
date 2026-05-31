# CLI Plugins Reference

Start with the [root plugin overview](../PLUGINS.md) for the model and the manifest schema. This doc covers the CLI surface: installation flags and errors, the `plugins` subcommand tree, the JSONL dispatch protocol your plugin's binary must speak, and the on-disk layout.

## Installing plugins

```
objectiveai plugins install --owner <o> --repository <r> [--commit-sha <s>] [--upgrade] [--allow-untrusted]
```

| Flag | Meaning |
|---|---|
| `--owner` | GitHub owner (org or user). Matched against the install whitelist. |
| `--repository` | GitHub repository name. Becomes the plugin's identifier on disk. The literal `objectiveai` (case-insensitive) is reserved. |
| `--commit-sha` | Pin the manifest fetch to a specific commit. Defaults to the repo's default branch (`HEAD`). |
| `--upgrade` | Replace an existing install. Without this flag, install refuses when `<plugins_dir>/<repository>.json` already exists. With it, the existing binary, `viewer/`, and `.json` are deleted before the new install runs. Extra runtime data under `<repository>/` is preserved either way. |
| `--allow-untrusted` | Bypass the install whitelist. Emits a `warn`-level notification before proceeding. |

### Errors

| Error | When |
|---|---|
| `ReservedRepositoryName` | `--repository` is `objectiveai` (any casing). |
| `PluginNotWhitelisted` | The (owner, repository, commit_sha or HEAD, manifest.version) tuple doesn't match any whitelist entry, and `--allow-untrusted` was not passed. |
| `AlreadyInstalled` | A sibling `.json` exists at `<plugins_dir>/<repository>.json` and `--upgrade` was not passed. |
| `ManifestBadStatus` / `BinaryBadStatus` / `ViewerZipBadStatus` | The corresponding GitHub asset returned a non-2xx status. |
| `ManifestParse` | The fetched `objectiveai.json` didn't deserialize. |
| `BinaryWrite` / `ManifestPersist` / `ViewerZipExtract` / `Chmod` | A disk-write step failed. |

The full enum lives in [`objectiveai-rs/src/filesystem/plugins/install_error.rs`](../objectiveai-rs/src/filesystem/plugins/install_error.rs).

### Transactional semantics

The install pipeline is:

1. Platform check (no disk touches).
2. Network phase — fetch the manifest, the platform binary, and the viewer zip (if declared) into memory.
3. Pre-flight refusal if `--repository` is reserved or already installed (without `--upgrade`).
4. If `--upgrade`, delete the prior install's three artifacts (binary, `viewer/`, manifest sibling).
5. `tokio::try_join!` three concurrent writes: binary + chmod, viewer-zip extract, manifest persist.

A network failure leaves the disk in whatever state step 4 left it — empty if `--upgrade`, untouched if a fresh install. A write-phase failure can leave partial new state; re-running with `--upgrade` cleans up.

### Install whitelist

Default whitelist allows the `ObjectiveAI` GitHub org only:

```
{ owner: "(?i)ObjectiveAI", repository: ".*", commit_sha: ".*", version: ".*" }
```

Each field is a regex matched **anchored** (`^…$`). The owner pattern uses `(?i)` for case-insensitive matching because GitHub usernames are case-insensitive.

## Listing and inspecting

```
objectiveai plugins list [--offset 0] [--limit 100]
```

Returns the names of installed plugins that have a `viewer_zip` (i.e. those that would surface as a viewer tab). Sorted by manifest mtime descending. Pagination matches the convention used by the logs list commands.

```
objectiveai plugins get <name>
```

Returns the full persisted manifest for one plugin (a `ManifestWithNameAndSource`), or `{"plugin": null}` if no manifest exists at `<plugins_dir>/<name>.json`.

## Running plugins

Two equivalent forms:

```
# Top-level catch-all — any unknown top-level subcommand is treated as a plugin.
objectiveai <name> <args…>

# Namespaced — explicit form, useful when the plugin name shadows a built-in.
objectiveai plugins <name> <args…>
```

`<args…>` is passed verbatim. The shell tokenizes; clap doesn't parse flags inside these dispatched calls. So:

```
objectiveai psyops --topic "wave physics" --steps 5
```

reaches the plugin's `main()` as `argv = ["psyops", "--topic", "wave physics", "--steps", "5"]`.

## The dispatch protocol

When the host dispatches your plugin, it spawns the binary at `<plugins_dir>/<repo>/plugin[.exe]` with:

- **argv** = `[binary_path, ...trailing_args]` (the same args the user typed).
- **stdin** = `/dev/null` (no stdin protocol yet).
- **stderr** = forwarded raw to the host's stderr.
- **stdout** = parsed one line at a time as JSONL `PluginOutput`.

### PluginOutput variants

Internally tagged on `"type"` (snake_case). Defined in [`objectiveai-sdk-rs/src/cli/plugins/output.rs`](../objectiveai-sdk-rs/src/cli/plugins/output.rs).

```jsonc
// 1. A notification — the value's fields sit flat next to the type discriminator.
//    The host wraps it in {"value": ...} when re-emitting to the user's JSONL stream.
{"type": "notification", "hello": "world"}

// 2. An error.
{"type": "error", "level": "error", "fatal": false, "message": "..."}
//   level   ∈ {"info", "warn", "error"}
//   fatal   = true terminates the host with exit code 1; false is informational

// 3. A command — the host re-spawns itself with this command, fire-and-forget.
{"type": "command", "command": "agents list --remote github"}
```

Unparseable lines (anything that isn't valid JSON or doesn't match a `PluginOutput` variant) are forwarded as string-valued notifications to the host's output — they still appear in the stream rather than being silently dropped.

### Smallest compliant plugin

[`objectiveai-cli/test-fixtures/hello-plugin/`](test-fixtures/hello-plugin/) is the minimal example: a single `main.rs` that reads `argv[1]` and emits one `{"type":"notification","hello":"<arg>"}` line. Used by the cli's e2e dispatch test.

## Filesystem layout

After `objectiveai plugins install --owner X --repository my-plugin`:

```
<plugins_dir>/                        (~/.objectiveai/plugins on Unix)
├── my-plugin.json                    ← persisted ManifestWithNameAndSource (name + source URL + manifest)
└── my-plugin/
    ├── plugin                        ← native binary (plugin.exe on Windows; 0o755 on Unix)
    ├── viewer/                       ← optional, only if viewer_zip declared
    │   ├── index.html
    │   └── assets/…
    └── …                             ← any other files the plugin's runtime created
                                        (preserved across `--upgrade`)
```

The cli's `resolve_plugin(name)` function looks at `<plugins_dir>/<name>/plugin[.exe]` for the dispatch target. The sibling `<name>.json` is what `plugins list` / `plugins get` consume.

Both files are tracked as "install data"; everything else under `<my-plugin>/` is "extra data" the plugin's runtime is welcome to use for state. `--upgrade` deletes the install data only.
