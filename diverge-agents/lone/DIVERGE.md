---
{
  "name": "lone",
  "chat": {
    "prompt_modalities": {
      "image": false,
      "audio": false,
      "file": false
    }
  },
  "structured_decisions": false,
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "title": "Agent",
    "description": "An agent that runs a LoNE model instead of calling an upstream\nmodel: a converted bundle mounted into the container, or a\n`lone serve` endpoint on the network.\n\nExactly one source. A bundle is the model on disk: a converted\nCore ML directory produced by `lone convert`, mounted at `bundle`,\nand every turn runs `lone harness` inside the container. An\n`endpoint` is that same harness driven as a thin client against a\nLoNE `serve` on the network: nothing is loaded in the container,\nso there is no Core ML and no macOS. A LoNE bundle carries its own\ntokenizer and chat template, so the system prompt, the sampling\nfields and whether the model thinks are the whole of what this\nagent can be told.\n\n# Secrets\n\nNone: the model is on disk or on the network, so there is no\ncredential. The template this agent is made from need declare no\nname, and a name it does declare is one the program never asks for.\n\n# The host\n\nA bundle turn needs Core ML and the Apple Neural Engine, so macOS.\nAn endpoint turn does not: the image installs the harness without\nCore ML, so it builds and registers anywhere, and a Linux provider\nreaches a LoNE host on the network. See the image's `/HARNESS.md`.",
    "type": "object",
    "properties": {
      "bundle": {
        "description": "The converted bundle's directory inside the container.\n\nMust hold the `manifest.json` `lone convert` writes. The\nregistration checks that before the agent counts as\nregistered; a missing bundle is refused where the caller can\nsee why. Exactly one of `bundle` and `endpoint` is required.",
        "type": [
          "string",
          "null"
        ]
      },
      "endpoint": {
        "description": "A LoNE `serve` base URL to run turns against, instead of a\nbundle.\n\n`lone harness --endpoint` posts the conversation and the tool\ndefinitions to `/v1/chat/completions` there; nothing is loaded\nin this container, so the turn needs neither Core ML nor macOS,\nand the server must reach a LoNE host on the network. Exactly\none of `bundle` and `endpoint` is required.",
        "type": [
          "string",
          "null"
        ]
      },
      "system_prompt": {
        "description": "The system prompt, sent as the conversation's leading message.\n\nAbsent sends none, and the bundle's own template is the whole\nof the framing.",
        "type": [
          "string",
          "null"
        ]
      },
      "max_new_tokens": {
        "description": "Cap on generated tokens for one turn, reasoning included.\n\nAbsent is the harness's 512; the model's context clamps it\nfurther when the conversation leaves less room.",
        "type": [
          "integer",
          "null"
        ],
        "format": "uint64",
        "minimum": 0
      },
      "temperature": {
        "description": "Sampling temperature. Absent is greedy, the harness's default.",
        "type": [
          "number",
          "null"
        ],
        "format": "double"
      },
      "top_p": {
        "description": "Nucleus sampling. Absent keeps the whole distribution.",
        "type": [
          "number",
          "null"
        ],
        "format": "double"
      },
      "top_k": {
        "description": "Top-k sampling. Absent keeps every token.",
        "type": [
          "integer",
          "null"
        ],
        "format": "uint64",
        "minimum": 0
      },
      "thinking": {
        "description": "Whether the model may think before answering. Absent is the\ntemplate's own default, which is yes.",
        "type": [
          "boolean",
          "null"
        ]
      },
      "dependencies": {
        "description": "The tool containers this agent depends on, each a dependency\ntool template the caller deploys and serves to it as an MCP\nserver, exactly as the SDK's [`Template`] states one: that\ntype's JSON Schema is what `--schema` prints for this member,\nin `/DIVERGE.md`, and this list, unchanged, is what the\nregistration answers — one schema, from the one type, at both.\nAbsent is none.",
        "type": "array",
        "items": {
          "$ref": "#/$defs/Template"
        }
      }
    },
    "$defs": {
      "Template": {
        "description": "One tool container an agent's program depends on: everything the\ncaller needs to deploy it, as fields and not as words. A tool\ncontainer depends on nothing.\n\nA [`Container`](crate::shared::containers::request::Container) is an\nimage, two limits, mounts and arguments. The image, the limits and\nthe arguments are here, member for member; beside them the secrets\nthe image needs, by name alone — which of them the tool's program\ntakes, and in what mode, is its own answer at registration, and\ntheir values are the caller's, each resolved through the vault key\nthe agent that spawns the tool binds the same name to, fed at every\nactivation. The mounts are not the caller's volumes — a volume is\nnamed in the caller's own listing, and a tool that named one would\nbe dictating the caller's storage — but, for a transient tool alone,\nthe AGENT's own files and directories, each a path in the agent\ncontainer served live at a path in the tool container: the caller\nserves the agent container, a `containers::fuse_serve` of the path,\nand mounts the tool, a `fuse::mount` answered from it.\n[`mode`](Self::mode) says which of the two modes the dependency is —\na transient tool of the agent's run, with those mounts, or a service\nthe caller keeps, with a scope — see [`Mode`]. What the tool may ask\nof the caller is not here: the tool's own program declares it at its\nregistration, every time it starts, as a standalone tool's does —\nsee the `permissions` of the\n[`register`](crate::container_proxy::inside::register) answer.\n\nThere are no instructions. What a template cannot say — the provider\nthe tool runs on, the volumes it is given — is\nthe caller's, and the caller needs no prose to choose it. The\ndaemon's own tool templates are another thing: a record the daemon\nkeeps, named by a hash, made by a request; this is what an image\ndeclares and a caller deploys.\n\nThere is no name. A dependency is named by its ID: the lowercase\nhexadecimal SHA-256 of its [`canonical`](crate::shared::canonical)\nbytes — this template as compact JSON, absent members omitted, the\nimage's `references` absent too, every object key sorted at every\ndepth, the `arguments` and the `secrets` included, the names sorted\nbytewise, the `mode` and\nits members among the keys — sixty-four characters, which the caller\ncomputes over [`hashed`](Self::hashed). Two declarations with one id\nare one dependency, and a list that declares one twice is refused;\nthe same template deployed for the same agent, on whatever provider,\nis the same dependency, and the same service template declared under\nthe same scope key, by whatever agent, is the same service. The\ncaller labels the tool's MCP tools by the id when it merges lists,\nand the program knows which tool serves each by the keys under\n`_meta`, see [`shared::mcp`](crate::shared::mcp).",
        "type": "object",
        "properties": {
          "image": {
            "description": "The image: a digest and its references, as a\n`containers::tools::run` request names one.",
            "$ref": "#/$defs/Image"
          },
          "memory": {
            "description": "The memory the tool container needs, in bytes.",
            "type": "integer",
            "format": "uint64",
            "minimum": 0
          },
          "disk": {
            "description": "The bytes the tool container may write, in bytes.",
            "type": "integer",
            "format": "uint64",
            "minimum": 0
          },
          "arguments": {
            "description": "The arguments the tool container is registered with — the\n`arguments` of the run request that makes it, verbatim."
          },
          "secrets": {
            "description": "The secrets the image needs, by name, in the order the image\nstates them: names only. Which of them the tool's program takes\nand in what mode is the program's answer at its registration —\nsee [`secrets`](crate::shared::containers::secrets) — and the\nvalue of each is resolved through the vault key the agent the\ntool is spawned for binds the same name to, fed at every\nactivation. Inside the hash, sorted bytewise there. A name is\nnot empty, no two are alike, and every one is a name the\nregistering agent's template declares; a template that says\notherwise is the registration's error. Absent, the image needs\nnone.",
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        },
        "required": [
          "image",
          "memory",
          "disk",
          "arguments"
        ],
        "oneOf": [
          {
            "description": "A tool for the agent's run: `\"mode\":\"transient\"`.",
            "type": "object",
            "properties": {
              "fuse_file_mounts": {
                "description": "Files of the agent container served live into the tool\ncontainer, one each over FUSE: see [`Mount`]. The agent's\npath names a file; the tool's path is where it appears.\nAbsent when empty.",
                "type": "array",
                "items": {
                  "$ref": "#/$defs/Mount"
                }
              },
              "fuse_directory_mounts": {
                "description": "Directories of the agent container served live into the\ntool container, one each over FUSE: see [`Mount`]. The\nagent's path names a directory, whose whole tree the tool\nsees; the tool's path is where it appears. Absent when\nempty.",
                "type": "array",
                "items": {
                  "$ref": "#/$defs/Mount"
                }
              },
              "mode": {
                "type": "string",
                "const": "transient"
              }
            },
            "required": [
              "mode"
            ]
          },
          {
            "description": "A service the caller keeps: `\"mode\":\"service\"`.",
            "type": "object",
            "properties": {
              "scope": {
                "description": "How far the service is shared, and its database with it:\nsee [`Scope`].",
                "$ref": "#/$defs/Scope"
              },
              "mode": {
                "type": "string",
                "const": "service"
              }
            },
            "required": [
              "mode",
              "scope"
            ]
          }
        ]
      },
      "Image": {
        "description": "The image, by its manifest digest and the references under which\nits bytes may be fetched — what\n[`images::oci`](crate::provider::endpoints::images::oci::client::request::Frame)\nasks about, so an oci scope that came back held names an image a\nrun can ask for, with nothing to translate between them. The same\nvalue is the whole of the shared\n[`oci::has`](crate::shared::containers::oci::has) request a\nprovider asks a caller on a run, and the image every\n[`oci::prove`](crate::shared::containers::oci::prove) ask on a\nscope naming it is over.\n\n# The digest is the image\n\nA digest cannot be repointed at different content, so an image\nmeans the same bytes every time, wherever they come from. That is\nthe whole of what the wire asserts: the provider runs the image\nthe digest names and no other. The references are not the image:\nthe same bytes sit under different names on different registries,\nand an id that hashes an image hashes it with none — see\n[`hashed`](Self::hashed).\n\n# Where the bytes come from\n\nA provider looks in the caller's own Store first, by digest alone —\nthe images the caller\n[built](crate::provider::endpoints::images::build) or\n[pushed](crate::provider::endpoints::images::push) on it, and no\nother caller's. Failing that it asks any referenced registry its\nown policy lists, with the credential it holds for it, and ignores\na reference naming one it does not; and failing that it takes the\nimage from the caller — asking on the run scope whether the caller\nholds it, naming this image, and, when it does, for its manifest\nand blobs, then for the proof that it holds it, see\n[`oci`](crate::shared::containers::oci). The order, and which\nanswered, is stated on\n[`Source`](crate::shared::containers::response::Source), which the\nrun's answer carries. A caller need not hold what it names; one\nthat does answers so when asked.",
        "type": "object",
        "properties": {
          "digest": {
            "description": "The manifest digest, `<algorithm>:<hex>`.\n\nWhat identifies the image: a runtime hashes what it pulls, so\na source serving other bytes under it fails before anything\nruns.",
            "type": "string"
          },
          "references": {
            "description": "Where the bytes may be fetched, each a registry and a path\nthere: see [`Reference`]. Absent when there are none, and then\nthe provider has only what it holds and what the caller holds.",
            "type": "array",
            "items": {
              "$ref": "#/$defs/Reference"
            }
          }
        },
        "required": [
          "digest"
        ]
      },
      "Reference": {
        "description": "One place the bytes an image's digest names may be fetched from: a\nregistry, and the repository path there. A reference locates and\ndoes not identify — the digest does — so an image may carry none,\none or several, and two images with one digest are one image\nwhatever each lists. A provider asks a referenced registry only\nwhere its own policy lists that registry, with the credential it\nholds for it, and ignores a reference naming one it does not.",
        "type": "object",
        "properties": {
          "registry": {
            "description": "The registry's host, as a reference names it before its first\n`/`: `docker.io`, `ghcr.io`, `registry.example.com:5000`. No\nscheme and no path.",
            "type": "string"
          },
          "name": {
            "description": "The repository path at that registry — `library/nginx`,\n`myorg/myimage`. No host and no tag: the image is\n`<registry>/<name>@<digest>` there.\n\nIt lands in a reference by concatenation wherever a provider\npulls from, so a provider refuses one that is not a repository\npath and normalizes nothing.",
            "type": "string"
          }
        },
        "required": [
          "registry",
          "name"
        ]
      },
      "Mount": {
        "description": "A path of the agent container, mounted at a path of the tool\ncontainer over FUSE: source before destination, the order a mount\nreads in everywhere else. Which of the two it is — a file or a\ndirectory — is which list of the [`Template`](super::Template) it\nis on.\n\nThe caller serves the agent container's path live — a\n`containers::fuse_serve` of it, or of the directory holding a file —\nand makes the mount in the tool container as a FUSE mount answered\nfrom that FUSE serve, so every read, write, listing, removal, rename and\nnew directory the tool makes is one ask answered from the agent's\nown filesystem, where the agent sees it at once. Nothing is copied:\nthe two containers share the path for as long as both run.",
        "type": "object",
        "properties": {
          "agent_path": {
            "description": "Where it is in the agent container, as path components from the\nagent container's root — the shape every path in this crate\ntakes. No component is empty, `.` or `..`, and the path is not\nthe root: a file on the file list, a directory on the directory\nlist.",
            "type": "array",
            "items": {
              "type": "string"
            }
          },
          "tool_path": {
            "description": "Where it appears in the tool container, as path components from\nthe tool container's root, under the rules a\n[`Container`](crate::shared::containers::request::Container)\nstates for its mounts: not the root, no other mount's, inside\nno other mount's, and no other mount's inside it.",
            "type": "array",
            "items": {
              "type": "string"
            }
          }
        },
        "required": [
          "agent_path",
          "tool_path"
        ]
      },
      "Scope": {
        "description": "What a service tool is one of: one per parent agent, one per\nparent agent template, or one for every agent that declares it,\nwhatever template they are made from. The scope names the service\n— the same template declared under the same scope key is the same\nservice, joined and not made again — and names its database scope\nwith it, so what a service holds is shared exactly as far as the\nservice is. Snake case on the wire: `\"instance\"`,\n`\"template\"`, `\"global\"`.",
        "oneOf": [
          {
            "description": "One service per parent agent: the service deployed for one\nagent is its own, with a database of its own, and two agents'\nservices of this template never see each other's. The scope a\ntransient tool has without saying so.",
            "type": "string",
            "const": "instance"
          },
          {
            "description": "One service per parent agent template: every agent made from\nthe same agent template that declares this template is\nattached to one service, which holds one database, so what one\nwrites the others read.",
            "type": "string",
            "const": "template"
          },
          {
            "description": "One service for the template, full stop: every agent that\ndeclares it, from whatever agent template, is attached to the\none service, and its database is one for all of them.",
            "type": "string",
            "const": "global"
          }
        ]
      }
    }
  }
}
---

# LoNE

An agent that runs a **LoNE model** — LFM3 / LFM2.5 converted for Core ML and the Apple Neural
Engine — instead of calling an upstream model. Exactly one source: a `bundle` mounted in the
container, or an `endpoint` naming a LoNE `serve` on the network. Every turn runs `lone harness`
over the whole conversation, and the chunks it writes are the turn.

A bundle turn needs **macOS** and Core ML. An endpoint turn does not: the harness posts the turn to
`/v1/chat/completions`, so the image runs on a Linux provider and the LoNE host does the inference.
The image installs the harness without Core ML (the manylinux wheel of `coremltools` does not exist
for arm64), so it builds and registers anywhere; a *bundle* turn on a Linux provider exits with
`lone targets the Apple Neural Engine and needs macOS` on stderr, which the loop reports as the
turn raising.

## Arguments

| member | required | meaning |
|---|---|---|
| `bundle` | one of | the converted bundle's directory **inside the container**, holding the `manifest.json` `lone convert` writes. Checked at registration; a path without it is refused |
| `endpoint` | one of | a LoNE `serve` base URL (`host:8082`, `…/v1`, or the full `/v1/chat/completions`). Nothing is loaded in the container and nothing local is checked; the server does the inference. Exactly one of `bundle` and `endpoint` is required |
| `system_prompt` | no | the conversation's leading system message |
| `max_new_tokens` | no | cap on generated tokens for one turn, reasoning included; absent is 512, and the bundle's context clamps it further |
| `temperature` | no | sampling temperature; absent is greedy |
| `top_p`, `top_k` | no | nucleus / top-k; absent keep the whole distribution |
| `thinking` | no | whether the model may think before answering; absent is the template's default (yes) |
| `dependencies` | no | tool containers, as in every agent-loop image |

Secrets: **none**. The model is on disk or on the network, so there is no credential, and the
template this agent is made from need declare no name.

## The bundle

Convert it where `lone` and the checkpoint live, then mount the result:

```bash
lone convert --model LFM3-1.7B --out bundles/lfm3-1.7b --ctx 2048
```

A full bundle is roughly twice the fp16 checkpoint because the decode and prefill shapes store
their weights separately; `--decode-only` halves it at the cost of slow prompts. Nothing is
installed at registration, so a bundle is mounted, not built into the image.

## The endpoint

On a LoNE host, serve the bundle to the network:

```bash
lone serve --bundle bundles/lfm3-1.7b --host 0.0.0.0 --port 8082
```

The agent's `endpoint` is that URL (`http://lfm3-host:8082`, or `…/v1`, or the full
`/v1/chat/completions`). The container needs no mount and no Core ML; each turn is one HTTP POST.
The host keeps the bundle loaded, and one request runs at a time.

## Minimal template body

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/lone" }] },
  "memory": 8589934592,
  "disk": 1073741824,
  "arguments": { "bundle": "/models/lfm3-1.7b", "temperature": 0.0, "max_new_tokens": 512 }
}
```

Or, against a served bundle, with no mounts:

```json
{
  "type": "agent",
  "image": { "digest": "<digest>", "references": [{ "registry": "docker.io", "name": "diverge-local/lone" }] },
  "memory": 8589934592,
  "disk": 1073741824,
  "arguments": { "endpoint": "http://lfm3-host:8082", "temperature": 0.0, "max_new_tokens": 512 }
}
```

The bundle lives on the LoNE host in that mode; nothing is mounted into the agent.

A template carries no mounts. Mount the bundle when the agent is made:

```json
{
  "template": "<id>",
  "name": "lone-1.7b",
  "provider": {
    "identity": { "issuer": "https://identity.diverge.network", "subject": "github.…" },
    "volume_mounts": [
      { "volume_name": "models", "volume_relative_path": ["lfm3-1.7b"], "volume_mode": "persistent", "container_path": ["models", "lfm3-1.7b"] }
    ]
  }
}
```

`memory` is a starting point: the 1.7B LUT8 stack measures ~3.5 GB on disk and the ANE runtime
holds the weights once. The mount is the bundle directory, read-only in effect: nothing writes to
it.

## The turn

The harness contract is the python image's (`diverge-agents/python/HARNESS.md`), carried to a
bundle or an endpoint: `{input, tools, resources}` on stdin, `{"eval": <chunks>, "stdout": ""}` as
the last stdout line. The history is replayed through the bundle's own chat template (on the LoNE
host, when the source is an endpoint), and the generated turn is split into `assistant_reasoning`,
`assistant_text_content` and `assistant_tool_call` chunks. Tool
calls are the template's own grammar, parsed back:

    <tool_call><function=name><parameter=k>v</parameter></function></tool_call>     LFM3
    <|tool_call_start|>[name(k='v')]<|tool_call_end|>                             LFM2.5

The image's `HARNESS.md` records the settled decisions.
