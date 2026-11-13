---
name: diverge-images
description: Container images in Diverge — what an image reference is (`{digest, references:[{registry,name}]}`), where a provider gets an image from (its Store, its podman cache, a listed registry, the caller), how to build an image on a provider with `diverge run images build` and get its digest, the image's `/DIVERGE.md` (frontmatter `name`, `schema`, `frontend`), the `diverge run images info` and `diverge run images oci` commands, and a reference of every image in the repository — the six agent-loop images (python, lone, openrouter, codex, hermes, cc) and the example images (courier, taskboard, memory, alarm, chat, human) with each one's `arguments`, secrets and their modes (fixed/rotating), dependencies and a minimal template body. Load it when you need an image digest for a template, when a deploy fails with the image unavailable or "the provider pulls from no registry the image's references name", when you need to know what `arguments` or `secrets` an image takes, when you build or rebuild an image, or when you read an image's DIVERGE.md.
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
  remote daemon holds the image. An image built with `diverge run images build` is in the Store and is
  found this way.

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

## Building an image

`diverge run images build` builds an image from a Containerfile on a provider, into the daemon's Store
there. This is the only way to build one: the provider's container runtime is the provider's own, and
an image put into it by any other means is not in the Store.

```bash
diverge run images build '{
  "provider": {"issuer": "http://127.0.0.1:14981", "subject": "provider"},
  "containerfile": "FROM docker.io/library/alpine\nCOPY DIVERGE.md /DIVERGE.md\n",
  "description": "my image",
  "context": {"volume": "my-source", "path": ["tool"]}
}' > build.log
# {"output":"[1/2] STEP 1/6: FROM …"}        one line per line the build says
# {"built":{"digest":"sha256:…","bytes":7675239}}
```

- **`provider`**: which provider builds and holds the image, by its identity (`diverge run providers list`).
- **`containerfile`**: the Containerfile's whole text, not a path.
- **`context`**: a directory of a volume on that same provider, as `{"volume":"<name>","path":[…]}`. The
  Containerfile's `COPY` and `ADD` read from it. Omit it for a Containerfile that copies nothing.
- **`description`**: optional, one short line (132 characters at most), kept beside the image.
- **The last line is the result**:
  - `{"built":{"digest":…,"bytes":…}}`: the image is in the Store. The digest is what a template names.
  - `{"exists":{"digest":…}}`: the build produced a digest already in the Store; nothing was added.
  - `"insufficient_capacity"`: the Store is full. `"context_held"`: something has the context volume to
    itself. `"unsupported"`: the provider has no Store (no `images` in its `provider.yaml`).
  - `"forbidden"`: no `build` grant over images, or no `mount` grant over the context volume.
    `"not_found"`: no such provider, or no such volume on it.
  - `{"error":…}`: the build failed; the `output` lines before it say why.
- A build cannot be cancelled. Stopping the command leaves it running on the provider.
- Each image that compiles Rust takes roughly ten minutes. Run builds one at a time, in the background,
  with output to a file.

### Getting source in front of a build

A build reads its context from a volume. For source on the provider's own host, list the folder as a
fixed, read-only volume in `provider.yaml` and restart the provider:

```bash
diverge provider config patch '[{"op":"add","path":"/volumes/fixed/-","value":
  {"name":"my-source","path":"C:/path/to/source","bytes":34359738368,"mode":"read_only"}}]'
```

The provider needs a Store to build into. If `diverge run providers list` shows `"images_store":false`:

```bash
diverge provider config patch '[{"op":"add","path":"/images","value":
  {"path":"C:/Users/me/.diverge/provider/images-store","capacity":34359738368}}]'
```

Otherwise create a volume and upload the source to it (`diverge-files`).

### Rules

- **A Containerfile sees only its context.** One that copies from several folders needs a context that
  holds them all: the Containerfiles in this repository (`diverge-agents/*/Containerfile`,
  `diverge-examples/*/*/Containerfile`) take the repository root as their context.
- **The Store is the daemon's.** An image built here runs for this daemon on that provider with nothing
  pulled. Another daemon does not see it.
- **Rebuild images whenever the inside-container contract changes** (the SDK's
  `container_proxy::inside`, the container proxy, or the loop crate). An image built against an older
  contract deploys but misbehaves against the current proxy (refused enqueues, a channel that finishes
  with no answer). The container proxy itself is never part of an image; the provider mounts it in.
- **A rebuild is a new digest.** Templates name digests, so a rebuilt image needs a new template.

Then use the digest in a template (see `diverge-agents` / `diverge-tools`). An image the daemon holds in
a Store is found by its digest, so it needs no `references`:

```json
{"type":"agent",
 "image":{"digest":"sha256:…"},
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
  `{"prompt_modalities": {…}, "instruction_modalities": {…}, "choice_modalities": {…}}`, each the same
  three booleans: what the decision's `content`, its questions' `instructions`, and its choices (option
  descriptions and score levels) may carry beside text.
- An agent's image declares at least one of the two; a tool's image states neither.
- **The document is required.** `agents create`, `standalone-tools create` and an agent's dependency
  declaration each read the image's document and refuse an image that ships none, that nobody in
  reach holds, or whose document does not parse. An agent is also refused when its image declares
  neither `chat` nor `structured_decisions`. A template is not checked: `templates create` takes any
  image.
- **Both are enforced** on `agents message`: `chat` for a request of type `chat`, and
  `structured_decisions` for one of type `structured_decisions` (see `diverge-agents`). A request for a mode the image says
  `false` to is refused by the daemon and never reaches the container.
- **What a program is handed.** Its `POST /run` body is tagged: `{"type":"chat","messages":[…]}` or
  `{"type":"structured_decisions","key":…,"content":[…],"questions":[…]}`. A program that declares
  structured decisions answers that run by echoing the decision as one `user_structured_decisions` chunk,
  saying its answers in one `structured_decisions_response` chunk, and ending the stream. `POST /enqueue`
  is chat only: the proxy never enqueues a decision, and hands a program nothing while a decision's run is
  open.
- **Echo what you take.** A program says a request back as it takes it up: a chat message as its user
  parts, a decision as its `user_structured_decisions` chunk, each under the `key` it was handed. The echo
  is how the daemon logs the request with its sender, and how a sender watching (`"watch":true`) finds its
  request in the log. A request accepted and never echoed is answered `"dropped"` to a watcher.
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
| `"no_source"` | no container of the image runs, and no connected provider or remote daemon holds it |
| `"forbidden"` | the account holds no `{"images":["info"]}` grant |
| `{"error":…}` | a failure, in the provider's words where it said any; nothing is retried |

How the daemon finds it, in order:

1. Its own record of the digest. Once a document has been found for a digest it is recorded and
   answered from the record from then on (a digest never changes what it ships). Only a document is
   recorded: a `missing`, a `no_source` and an `error` are asked again.
2. A running container of that digest (an agent's or a tool's the daemon runs itself): one is chosen
   and asked on its run's info channel. References are not compared, and the asker's grants over
   agents or tools do not narrow this.
3. Otherwise every connected provider and every remote daemon is asked **at once** whether it holds
   the image (the cheap question `images oci` asks; nothing is pulled). The first that does is asked
   for the document: a provider reads the file out of the image **without running it**; a remote
   daemon answers `images info` itself, by these same rules. One that fails is passed over for the
   next holder.
4. With no holder, every connected provider is asked for the document outright, all at once. This
   finds an image that is only in a provider's local podman cache, which the "holds" question does
   not look in.
5. Nothing found is `no_source`, or the last failure heard.

Notes:

- A remote daemon is asked as the account this daemon holds there, which needs `{"images":["info"]}`
  on that daemon (and the `oci` grant for the "holds" question). Without it that daemon is passed over.
- The request's optional `ignore` (identities of daemons not to ask) is the loop guard daemons use
  among themselves; leave it out.
- **`agents create` and `standalone-tools create` run this search** for the template's image and
  refuse an image with no document, so the first create of a new image can take as long as a
  provider needs to pull it. Asking `images info` beforehand records the document, and the create
  then answers from the record.

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
| `diverge-agents/lone` | `lone` | agent, local LoNE bundle (macOS) | `bundle` | none |
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

- [references/agent-loop-images.md](references/agent-loop-images.md) — python, lone,
  openrouter, codex, cc, and the shared `dependencies` member.
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
| Start error saying the image is unavailable | The digest is not in the daemon's Store on that provider and no listed registry serves it. Build it with `diverge run images build` on that provider and use the digest it prints. |
| The image deploys but its program misbehaves against the proxy | The image predates the current inside-container contract. Rebuild it from current source. |
| Start error naming a secret the template does not declare | The program's arguments select a secret (see the per-image reference) that the template's `secrets` omits. Add the name to the template (a new template id) and bind it on the create. |
| `images info` answers `no_source` for a local image | No connected provider has the image in its cache, its Store or a listed registry, and no remote daemon holds it. Build it on a connected provider with `diverge run images build`. |
| A rebuilt image has the same tag but templates still run the old one | Templates name digests. A rebuild is a new digest: make a new template with it. |

## Not verified

- The provider protocol also has `images::push`, `edit`, `stat`, `list` and `delete` over the caller's
  Store, but the CLI exposes only `build`, `info` and `oci` under `diverge run images`. Listing or
  deleting what the Store holds from the CLI is therefore not covered.
- Running a container from a Store image whose template names no `references` is from the daemon's
  code (it asks who holds the digest when no registry matches), not run live.
- macOS and Linux build commands are from code and the setup report, not run live.
- `diverge-agents/cc` declares no secret and says Claude Code logs in by itself; how a login
  reaches the container (for example a mount of its credential directory) was not established.
- Suggested `memory` / `disk` values in the references are starting points, not measured minimums.

## When this task is done

Load the `diverge` skill again before starting the next Diverge task: it is the index, and it says which
skill the next task needs.
