---
name: diverge-images
description: Container images in Diverge — what an image reference is (`{digest, references:[{registry,name}]}`), where a provider gets an image from (its Store, its podman cache, a listed registry, the caller), how to build an image into the provider's podman store and read its digest, the image's `/DIVERGE.md` (frontmatter `name`, `schema`, `frontend`), the `diverge run images info` and `diverge run images oci` commands, and a reference of every image in the repository — the five agent-loop images (python, openrouter, codex, hermes, cc) and the example images (courier, taskboard, memory, alarm, chat, human) with each one's `arguments`, secrets and their modes (fixed/rotating), dependencies and a minimal template body. Load it when you need an image digest for a template, when a deploy fails with the image unavailable or "the provider pulls from no registry the image's references name", when you need to know what `arguments` or `secrets` an image takes, when you build or rebuild an image, or when you read an image's DIVERGE.md.
---

# Diverge images

An image is an OCI container image whose program speaks Diverge's inside-container contract: an agent
loop (it takes messages and answers them) or a tool (an MCP server). Diverge never names an image by
tag. A template names it by **manifest digest**, plus a list of **references** that say where the bytes
may be fetched. The daemon keeps no record of images; it reaches one through a container that runs it
or a provider that can obtain it.

This skill covers: the image reference, where a provider finds an image, building one into the
provider's store, `/DIVERGE.md`, the two `images` commands, and what each image in the repository
takes. Templates, agents and tools themselves are in `diverge-agents` and `diverge-tools`; vault keys
for secrets are in `diverge-storage`.

## The image reference

Every place that names an image (an agent template, a tool template, a dependency entry,
`images info`, `images oci`) uses the same object:

```json
{
  "digest": "sha256:9f2c…",
  "references": [
    { "registry": "docker.io", "name": "diverge-local/python" }
  ]
}
```

- `digest` — the manifest digest, `<algorithm>:<hex>`. **The digest is the image.** A provider runs
  the bytes the digest names and nothing else; the runtime hashes what it pulls.
- `references` — zero or more places the bytes may be fetched. `registry` is a bare host
  (`docker.io`, `ghcr.io`, `registry.example.com:5000`; no scheme, no path). `name` is the repository
  path there (`library/nginx`, `myorg/myimage`; no host, no tag). The image is
  `<registry>/<name>@<digest>` at that registry. `name` must be a valid lowercase repository path;
  nothing is normalized.
- References locate and do not identify. Two images with one digest are one image whatever each
  lists, and a template's id hashes the image **without** its references.

### The rule that trips everyone: references must name a registry the provider lists

Before the daemon asks a provider to run a container, it checks the provider's capabilities
(`images_registries`, visible inside a connected record's `status` in `diverge run providers list`,
under its `info`'s `capabilities`). A provider passes when
**some reference of the image names a registry that provider lists**. The default provider lists
`docker.io`, `ghcr.io` and `quay.io`.

- An image with **no references** fails this test ("the image names no registry").
- An image whose references name only unlisted registries fails it ("the provider pulls from no
  registry the image's references name").
- When no connected provider passes, the daemon falls back to asking whether a provider's Store or a
  remote daemon holds the image. For an image you built by hand into the provider's podman cache that
  fallback finds nothing, so the start fails.

So even for an image that exists only in the provider's local cache, put a reference naming a listed
registry in the body, for example `{"registry":"docker.io","name":"diverge-local/python"}`. Nothing is
pulled from that registry: the provider finds the digest locally first (next section).

## Where a provider gets an image

For a run, the provider looks in this order and takes the first that has the digest:

1. **The caller's Store.** A digest the calling daemon owns in the provider's image Store (one built
   or pushed on that provider through the provider protocol). Answered as source `store`.
2. **The provider's podman cache, then a listed registry.** If podman already holds an image with
   that digest under a name that does **not** start with `127.0.0.1:`, it is run as it is with nothing
   pulled. Otherwise every reference that names a registry in the provider's configuration is asked
   at once (`<registry>/<name>@<digest>`, with the credential the provider holds for it) and the
   first that serves it is pulled. References naming unlisted registries are ignored. Answered as
   source `{"registry":{"reference":"…"}}`.
3. **The caller.** The provider asks the daemon whether it holds the image; the daemon may supply it
   from another provider's Store or a remote daemon over OCI channels, and must then pass a nonce
   proof. Answered as source `client`; a failed proof fails the run.
4. Otherwise the image cannot be supplied and the start fails.

A registry is preferred to the caller, and the caller's Store to both. The provider tells the daemon
which source answered; no CLI answer reports it, so read it off the order above.

## Building an image into the provider's store

This is the normal way to make a local image usable: build it **inside the podman the provider
drives**, under an ordinary name, and pass its digest. The provider must be running (on Windows and
macOS that is what keeps its podman machine up).

```bash
# Windows / macOS: the provider drives a podman machine named diverge-provider through the
# rootful connection diverge-provider-root, with its data under <root>/provider/podman_data.
PD="$HOME/.diverge/provider/podman_data"; command -v cygpath >/dev/null && PD="$(cygpath -w "$PD")"
pp() { XDG_DATA_HOME="$PD" XDG_CONFIG_HOME="$PD" podman --connection diverge-provider-root "$@"; }
# Linux (the provider runs as root, no machine):
# pp() { sudo podman --root "<root>/provider/podman_data" "$@"; }

cd <workspace>                       # the repository root: it is the build context for every image
pp build --label diverge.network/keep=true \
   -t localhost/diverge-local/python:dev \
   -f diverge-agents/python/Containerfile . > build-python.log 2>&1

pp image inspect --format '{{.Digest}}' localhost/diverge-local/python:dev     # sha256:…
pp image ls --no-trunc --format '{{.Repository}}:{{.Tag}} {{.Digest}}'          # every image
```

Rules:

- **Build context is the workspace root (`.`)** for every Containerfile in the repository
  (`diverge-agents/*/Containerfile`, `diverge-examples/agents/*/Containerfile`, `diverge-examples/tools/*/Containerfile`). Each copies `diverge-sdk-rs/`,
  its own crate and `Cargo.lock`.
- **Never tag an image under `127.0.0.1:`**. The cache lookup skips those names on purpose (they are
  the provider's own per-pull registry names).
- **`--label diverge.network/keep=true`** protects the image from the provider's cache eviction.
  Images the provider did not record itself are evicted first once the cache passes the configured
  `image_cache_disk`.
- **Use the provider's podman, not your default one.** A plain `podman build` on Windows/macOS goes to
  a different machine and the provider never sees the image. Always go through the `pp` helper.
- **The digest must be recorded by podman.** The lookup is `podman image ls --filter digest=<d>`; an
  image loaded with no digest recorded is not matched.
- Each image compiles Rust inside the build and takes roughly ten minutes. Run builds one at a time,
  in the background, with output to a file.
- **Rebuild images whenever the inside-container contract changes** (the SDK's
  `container_proxy::inside`, the container proxy, or the loop crate). An image built against an older
  contract deploys but misbehaves against the current proxy (refused enqueues, a channel that finishes
  with no answer). The container proxy itself is never part of an image; the provider mounts it in.
- On Git Bash, `podman exec <ctr> /some/path` has its path rewritten; prefix that one command with
  `MSYS_NO_PATHCONV=1`.

Then use the digest in a template (see `diverge-agents` / `diverge-tools`):

```json
{"type":"agent",
 "image":{"digest":"sha256:…","references":[{"registry":"docker.io","name":"diverge-local/python"}]},
 "memory":536870912,"disk":1073741824,
 "arguments":{"python":"[{\"type\":\"assistant_text_content\",\"text\":\"hello\"}]"}}
```

## `/DIVERGE.md`: the image's own word on what it takes

Every image ships one file at the root of its filesystem, `/DIVERGE.md`. It is how a caller learns
what `arguments` an image accepts without knowing the image.

```text
---
{
  "name": "chat",
  "schema": { "type": "object", "additionalProperties": false, "description": "…" },
  "frontend": true
}
---
# Chat

Prose: what the image runs, its tools, the secrets it declares, how to run it.
```

The parse rules, in the order they are checked:

1. The first line is exactly `---` (a line ends at `\n`; a trailing space or a carriage return makes
   it not a fence). Otherwise there is no frontmatter.
2. The frontmatter is every line up to the next line that is exactly `---`. No such line is an
   unterminated frontmatter.
3. The frontmatter is **one JSON object** (JSON is valid YAML flow style) with these members and no
   other: `name` (string, required), `schema` (required), `frontend` (boolean, optional, `false` when
   absent), and, for an agent's image, `chat` and `structured_decisions` (each `false` or an object,
   never `true`; optional, `false` when absent). A missing member, an extra member or a wrong type is
   an error.
4. `schema` must be a JSON object: it is the **JSON Schema of the image's `arguments`**, not of the
   whole template.
5. The body is everything after the closing fence, verbatim.

What each member means:

- `name` — what the image calls itself. Nothing makes it unique.
- `schema` — what the template's `arguments` value may be. The daemon and the provider carry
  `arguments` to the program unread; the program validates them at its registration, so arguments
  that break the schema surface as a start error in the agent's or tool's log, not at
  `templates create`.
- `frontend` — `true` when the program serves a web UI beneath `/frontend` (see the `frontend`
  commands in `diverge-agents` / `diverge-tools`).
- `chat` — an agent's image only. `false`, or `{"prompt_modalities": {"image": …, "audio": …, "file": …}}`:
  what a message to the agent may carry beside text, all three booleans required. Text is always
  taken and is not listed. `file` is an embedded resource that is neither an image nor audio.
- `structured_decisions` — an agent's image only. `false`, or
  `{"prompt_modalities": {…}, "choice_modalities": {…}}`, each the same three booleans.
- An agent's image declares at least one of the two; a tool's image states neither.
- **The document is required.** `templates create` (agents and standalone tools) and an agent's
  dependency declaration each read the image's document and refuse an image that ships none, that no
  connected provider can obtain, or whose document does not parse. An agent template is also refused
  when its image declares neither `chat` nor `structured_decisions`.
- **`chat` is enforced** on `agents message`: see `diverge-agents`. `structured_decisions` is carried
  and not yet acted on.
- The body states, in prose, the tools the image serves and the **secrets it declares** with their
  modes. Read it before writing a template.

The images in this repository generate their schema from their Rust argument types; running the
image's program with `--schema` prints the same schema as the frontmatter carries.

## `diverge run images info`: read an image's DIVERGE.md

When: you have a digest and need the argument schema or the secrets an image declares.

```bash
diverge run images info '{"image":{"digest":"sha256:…","references":[{"registry":"docker.io","name":"diverge-local/python"}]}}'

# just the argument schema, or just the prose
diverge run images info --jq '.info.schema' '{"image":{…}}'
diverge run images info --jq '.info.body'   '{"image":{…}}'
```

Body: `image` (required, the reference above) and the optional `relay`. Run
`diverge run images info request-schema` for the exact shape.

One answer:

| answer | meaning |
|---|---|
| `{"info":{"name":…,"schema":{…},"frontend":true,"chat":{…},"body":"…"}}` | the document, parsed (`frontend`, `chat` and `structured_decisions` each absent when false) |
| `"missing"` | the image ships no `/DIVERGE.md` |
| `"no_source"` | no container of the image runs and no connected provider can obtain it; nothing was asked |
| `"forbidden"` | the account holds no `{"images":["info"]}` grant |
| `{"error":…}` | a failure, in the provider's words where it said any; nothing is retried |

How the daemon finds it, in order:

1. Its own record of the digest. Once an `info` or a `missing` has been found for a digest it is
   recorded and answered from the record from then on (a digest never changes what it ships).
2. A running container of that digest (an agent's or a tool's the daemon runs itself): one is chosen
   and asked on its run's info channel. References are not compared, and the asker's grants over
   agents or tools do not narrow this.
3. Otherwise a connected provider that lists a registry one of the references names (the same test a
   run makes), or failing that a provider whose Store holds the image. That provider reads the file
   out of the image **without running it**.
4. No provider either way is `no_source`.

Pitfalls: an image that is only in a provider's local cache needs a `references` entry naming a
listed registry, exactly as a template does, or the answer is `no_source`. A `no_source` or an
`error` is not recorded, so asking again later can succeed.

Grant: `{"images":["info"]}` (a bare array; there is no `within`, since images are not records).

## `diverge run images oci`: can the daemon supply an image, and one ask of it

When: you need to know whether any source the daemon reaches (a provider's Store or registries, or a
remote daemon) can supply an image by digest, or you want one of its manifests, one of its blobs, or
a possession proof. Rarely needed for ordinary use; it is the daemon-facing form of the OCI exchange
providers use to move images between each other.

```bash
# only the question
diverge run images oci '{"image":{"digest":"sha256:…","references":[{"registry":"docker.io","name":"library/alpine"}]}}'

# the question, then one ask on the held scope
diverge run images oci --manifest sha256:… '{"image":{…}}'
diverge run images oci --blob sha256:… --out layer.bin '{"image":{…}}'
diverge run images oci --prove-nonce <64 hex chars> [--platform sha256:…] '{"image":{…}}'
```

Body: `image` (required), `ignore` (optional list of `{issuer, subject}` identities of remote daemons
not to ask), `relay` (optional). The body is required here; there is no `{}` default that makes
sense. Flags, at most one ask per invocation:

| flag | does |
|---|---|
| `--manifest <DIGEST>` | prints the manifest under that digest the image reaches, with its media type |
| `--blob <DIGEST>` with `--out <PATH>` | writes that blob to the file (truncated first); `--blob` and `--out` require each other |
| `--prove-nonce <HEX>` | prints the proof over the image under that 64-hex-character nonce |
| `--platform <DIGEST>` | only with `--prove-nonce`: which manifest of an image index is meant |

Lines printed: first the daemon's answer, then (only after `"held"` and only when an ask was given)
the ask's answer.

| line | meaning |
|---|---|
| `"held"` | a source can supply the image; the scope is served through that one source |
| `"not_found"` | no source the daemon asked can |
| `"forbidden"` | the account holds `oci` over no provider and no remote daemon record |
| `{"error":…}` | a failure |
| `{"media_type":"…","body":"…"}` | the manifest asked for |
| `{"path":"…","bytes":N}` | the blob asked for, written |
| `{"proof":"<64 hex>"}` | the proof asked for |
| `"not_held"` | the source did not answer the ask: a digest the image does not reach, or a proof it would not give |

Rules:

- The daemon holds nothing itself. It opens the question on every connected provider that keeps a
  Store or lists a registry, and on every remote daemon it records, all at once; the first to answer
  held is the source and the rest are cancelled.
- Only sources the caller's grants reach are asked: a provider where a `providers` grant holds `oci`
  over it, a remote daemon where a `daemons` grant holds `oci` over its record. See `diverge-access`.
- A remote daemon in `ignore`, or the daemon itself, is never asked, so rings of daemons end.
- The CLI cancels the scope after the one ask; each invocation is a fresh question.

## The images in this repository

Each image's directory holds its `Containerfile` and its `DIVERGE.md`. Build any of them as shown
above with `-f <dir>/Containerfile .`.

| image dir | `name` | kind | required `arguments` | secrets the program declares |
|---|---|---|---|---|
| `diverge-agents/python` | `python` | agent, no model | `python` | none |
| `diverge-agents/openrouter` | `openrouter` | agent | `model` | `OPENROUTER_API_KEY` (fixed) |
| `diverge-agents/codex` | `codex` | agent | `model`, `auth` | one, by `auth.method`: `OPENAI_API_KEY`, `CODEX_ACCESS_TOKEN`, `OPENAI_IDENTITY_TOKEN` (fixed) or `CODEX_AUTH` (rotating) |
| `diverge-agents/hermes` | `hermes` | agent | `provider`, `model` | selected by the provider and the toolsets chosen |
| `diverge-agents/cc` | `cc` | agent (Claude Code) | `model` | none |
| `diverge-examples/tools/courier` | `courier` | dependency tool: service, `global` | `{}` | none |
| `diverge-examples/tools/taskboard` | `taskboard` | dependency tool: service, `template` (or `global`) | `{}` | none |
| `diverge-examples/tools/memory` | `memory` | dependency tool: transient | `{}` | none |
| `diverge-examples/tools/alarm` | `alarm` | dependency tool: service, any scope | `{}` | none |
| `diverge-examples/tools/chat` | `chat` | standalone tool: service, has a frontend | `{}` | none |
| `diverge-examples/agents/human` | `human` | agent, no model, has a frontend | `{}` | none |

Full per-image detail — every argument, every secret with its mode and value shape, dependencies and
a minimal working template body:

- [references/agent-loop-images.md](references/agent-loop-images.md) — python, openrouter, codex,
  cc, and the shared `dependencies` member.
- [references/hermes.md](references/hermes.md) — hermes: every provider id and its secrets, every
  toolset.
- [references/example-images.md](references/example-images.md) — courier, taskboard, memory, alarm,
  chat, human.

### How secrets work for an image (summary)

Three parties, three statements:

1. The **template** lists secret **names** (`"secrets":["OPENROUTER_API_KEY"]`).
2. The **agent or tool** made from it binds each name to a **vault key**
   (`"secrets":{"OPENROUTER_API_KEY":"my-or-key"}` on its create).
3. The **program**, at its registration, declares which of those names it takes and each one's
   **mode**: `fixed` (a value read at every activation, never locked) or `rotating` (one value locked
   to the container while it is active and written back when it goes inactive).

A program may take fewer names than the template declares, never more: a name the program declares
that the template does not is a start error. Secrets are never arguments and never environment the
caller sets. The vault side (keys, slots, locks, `use` grant) is in `diverge-storage`.

### How dependencies work for an image (summary)

Every agent-loop image accepts a `dependencies` argument: a list of dependency tool templates the
daemon deploys and serves to the agent as MCP servers. Each entry is
`{"image":{…},"memory":N,"disk":N,"arguments":{…},"secrets":[…],"mode":"transient"}` or
`…,"mode":"service","scope":"global"|"template"|"instance"`. A dependency's secrets are resolved
through the **agent's** binding, so every name a dependency lists must also be a name the agent's
template declares. Details and lifecycle are in `diverge-tools`; the entry shape is in
[references/agent-loop-images.md](references/agent-loop-images.md).

## Troubleshooting

| symptom | cause and fix |
|---|---|
| Start error "the provider pulls from no registry the image's references name" or "the image names no registry" | The image body has no `references` entry naming a registry the provider lists. Add one (`docker.io` for the default provider). |
| Start error saying the image is unavailable | The digest is not in the **provider's** podman store (it was built with another podman connection), or it is only tagged under `127.0.0.1:`, or no digest was recorded for it. Rebuild through the provider's podman and re-read the digest. |
| The image deploys but its program misbehaves against the proxy | The image predates the current inside-container contract. Rebuild it from current source. |
| Start error naming a secret the template does not declare | The program's arguments select a secret (see the per-image reference) that the template's `secrets` omits. Add the name to the template (a new template id) and bind it on the create. |
| `images info` answers `no_source` for a local image | No `references` entry names a listed registry and nothing of the image runs. Add the reference. |
| A rebuilt image has the same tag but templates still run the old one | Templates name digests. A rebuild is a new digest: make a new template with it. |

## Not verified

- The provider protocol has `images::build`, `push`, `edit`, `stat`, `list` and `delete` over the
  caller's Store, but the installed CLI exposes none of them under `diverge run images` (only `info`
  and `oci`). Putting an image into the Store from the CLI is therefore not covered; build into the
  provider's podman cache instead.
- macOS and Linux build commands are from code and the setup report, not run live.
- `diverge-agents/cc` declares no secret and says Claude Code logs in by itself; how a login
  reaches the container (for example a mount of its credential directory) was not established.
- Suggested `memory` / `disk` values in the references are starting points, not measured minimums.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
