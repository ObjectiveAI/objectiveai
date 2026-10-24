---
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "Agent",
  "description": "An agent running against Codex.\n\nFew knobs, and not because anything is missing: Codex decides its\nown sampling, its own tools and its own loop, and what it exposes\nto a caller is the model, how hard it reasons and how it speaks,\nwhether it may search the web, where it sends its requests, and\nhow it logs in \u00e2\u20ac\u201d the method only, never the credential, which is\nthe one secret the method declares (see [`Auth`]). Each field is\none config key or flag, named in its doc; every `Option` absent\nleaves that key unset, so Codex applies its own default for the\nmodel \u00e2\u20ac\u201d except [`web_search`](Self::web_search), where absent is\noff, because nothing is on by omission.",
  "type": "object",
  "properties": {
    "model": {
      "description": "The model to run: `model` (`--model`), in the endpoint's own\nnaming.",
      "type": "string"
    },
    "effort": {
      "description": "How much effort the model spends reasoning:\n`model_reasoning_effort`. See [`Effort`].",
      "anyOf": [
        {
          "$ref": "#/$defs/Effort"
        },
        {
          "type": "null"
        }
      ]
    },
    "reasoning_summary": {
      "description": "How much of its reasoning the model says out loud:\n`model_reasoning_summary`. What the stream's reasoning chunks\ncarry. See [`Summary`].",
      "anyOf": [
        {
          "$ref": "#/$defs/Summary"
        },
        {
          "type": "null"
        }
      ]
    },
    "verbosity": {
      "description": "How long the model's answers run: `model_verbosity`. See\n[`Verbosity`].",
      "anyOf": [
        {
          "$ref": "#/$defs/Verbosity"
        },
        {
          "type": "null"
        }
      ]
    },
    "web_search": {
      "description": "Whether, and how, the model may search the web: `web_search`.\nA capability of Codex itself rather than a tool of the\ncaller's \u00e2\u20ac\u201d Codex performs the search and never calls out.\nABSENT IS `disabled`. See [`WebSearch`].",
      "anyOf": [
        {
          "$ref": "#/$defs/WebSearch"
        },
        {
          "type": "null"
        }
      ]
    },
    "provider": {
      "description": "Where requests go instead of OpenAI: a `model_providers` entry\nthe harness writes and selects, keyed by `CODEX_API_KEY` \u00e2\u20ac\u201d so\nit requires the `api_key` login, and the registration refuses\nit beside another. Absent is OpenAI's own endpoint. See\n[`Provider`].",
      "anyOf": [
        {
          "$ref": "#/$defs/Provider"
        },
        {
          "type": "null"
        }
      ]
    },
    "auth": {
      "description": "How Codex logs in: one of its own methods, tagged `method`,\neach declaring the one secret the proxy feeds. REQUIRED \u00e2\u20ac\u201d no\nmethod is the default, because every one bills someone. See\n[`Auth`].",
      "$ref": "#/$defs/Auth"
    },
    "dependencies": {
      "description": "The tool containers this agent depends on, each a dependency\ntool template the caller deploys and serves to it as an MCP\nserver, exactly as the SDK's [`Template`] states one: that\ntype's JSON Schema is what `--schema` prints for this member,\nin `/DIVERGE.md`, and this list, unchanged, is what the\nregistration answers \u00e2\u20ac\u201d one schema, from the one type, at both.\nAbsent is none.",
      "type": "array",
      "items": {
        "$ref": "#/$defs/Template"
      }
    }
  },
  "required": [
    "model",
    "auth"
  ],
  "$defs": {
    "Effort": {
      "description": "How much effort the model spends before answering: Codex's\n`model_reasoning_effort`, its five rungs as Codex spells them.\nThere is no `max`: `xhigh` is the top, and a model that has fewer\nrungs takes the nearest it has.",
      "oneOf": [
        {
          "description": "Least.",
          "type": "string",
          "const": "minimal"
        },
        {
          "description": "Little.",
          "type": "string",
          "const": "low"
        },
        {
          "description": "Balanced.",
          "type": "string",
          "const": "medium"
        },
        {
          "description": "More.",
          "type": "string",
          "const": "high"
        },
        {
          "description": "As much as the model will spend.",
          "type": "string",
          "const": "xhigh"
        }
      ]
    },
    "Summary": {
      "description": "How much of its reasoning the model says out loud: Codex's\n`model_reasoning_summary`. What the run's reasoning chunks carry \u00e2\u20ac\u201d\nCodex streams the summary, never the raw reasoning, so this is\nthe whole of what a caller can see of the model thinking.",
      "oneOf": [
        {
          "description": "The model decides.",
          "type": "string",
          "const": "auto"
        },
        {
          "description": "Short.",
          "type": "string",
          "const": "concise"
        },
        {
          "description": "Long.",
          "type": "string",
          "const": "detailed"
        },
        {
          "description": "No summary: the reasoning chunks stay empty.",
          "type": "string",
          "const": "none"
        }
      ]
    },
    "Verbosity": {
      "description": "How long the model's answers run: Codex's `model_verbosity`,\nhonored by the models that take it (the reference names the GPT-5\nfamily on the Responses API) and unset otherwise.",
      "oneOf": [
        {
          "description": "Terse.",
          "type": "string",
          "const": "low"
        },
        {
          "description": "Balanced.",
          "type": "string",
          "const": "medium"
        },
        {
          "description": "Expansive.",
          "type": "string",
          "const": "high"
        }
      ]
    },
    "WebSearch": {
      "description": "Whether, and how, the model may search the web: Codex's\n`web_search`, its four values as Codex spells them. Codex performs\nthe search itself \u00e2\u20ac\u201d it is a capability of the upstream, not a tool\nthe caller serves \u00e2\u20ac\u201d so a search never appears on the stream as\none of the caller's tool calls. The harness writes `disabled` when\nthe agent says nothing.",
      "oneOf": [
        {
          "description": "No search at all.",
          "type": "string",
          "const": "disabled"
        },
        {
          "description": "Search over a cached index.",
          "type": "string",
          "const": "cached"
        },
        {
          "description": "Search over the live index.",
          "type": "string",
          "const": "indexed"
        },
        {
          "description": "Live fetches of the pages themselves.",
          "type": "string",
          "const": "live"
        }
      ]
    },
    "Provider": {
      "description": "An endpoint other than OpenAI's: one `model_providers` entry the\nharness writes as `model_providers.diverge` and selects with\n`model_provider = \"diverge\"`. The entry's `env_key` is\n`CODEX_API_KEY` \u00e2\u20ac\u201d the variable the `api_key` login method hands\nthe fed `OPENAI_API_KEY` to Codex under \u00e2\u20ac\u201d and its `wire_api` is\n`responses`, the one value the configuration reference names, so\nthe endpoint must speak the Responses API, which the Diverge relay\nand every upstream this protocol routes do.\n\nAbsent, Codex's own `openai` provider is used, with whatever\nlogin the agent's [`auth`](super::Agent::auth) names. Present, the\nentry REQUIRES the `api_key` method: a ChatGPT login speaks only\nto OpenAI's own backend, and no other method fills an `env_key`,\nso the registration refuses a provider beside any other method\n(`400`, `{\"kind\":\"auth\"}`).",
      "type": "object",
      "properties": {
        "base_url": {
          "description": "The base URL, `/v1` included: `model_providers.diverge.base_url`.",
          "type": "string"
        }
      },
      "required": [
        "base_url"
      ]
    },
    "Auth": {
      "description": "The login method, and through it the ONE secret the agent takes.\n\nThe agent names the method and never a credential: each method\ndeclares one secret, by a fixed name the template must declare,\nand the value the caller binds to that name is what Codex logs in\nwith. The four are Codex's own ways in, as its source has them\n(`rust-v0.163.0`, against the pinned `0.153.4`), and nothing the\nrun does with them is invented: a variable Codex reads, or a file\nat the path Codex reads.\n\n`OPENAI_API_KEY` in the environment is NOT one of them. Codex's\nbuilt-in `openai` provider has no `env_key` \u00e2\u20ac\u201d the variable only\nprefills the interactive login \u00e2\u20ac\u201d so a key in the environment under\nthat name logs nothing in. `CODEX_API_KEY` is the variable the\ncredential store reads first, and `codex exec` honours it; the\n`api_key` method is the fed `OPENAI_API_KEY` handed to Codex under\nthat name.\n\nA ChatGPT login \u00e2\u20ac\u201d the `auth_file` method with ChatGPT tokens in\nit \u00e2\u20ac\u201d speaks only to OpenAI's own backend, and a\n[`provider`](super::Agent::provider) beside one fails as itself,\nin Codex's words; a provider's entry is keyed by `CODEX_API_KEY`,\nso a provider REQUIRES `api_key`, and the registration refuses\nthe pair otherwise. Tagged `method`, snake case on the wire.",
      "oneOf": [
        {
          "description": "An OpenAI API key. Declares `OPENAI_API_KEY`, `fixed`: a JSON\nstring, put in the process environment as `CODEX_API_KEY`,\nthe top-precedence credential \u00e2\u20ac\u201d above `auth.json`, which is\nnever written under this method. Bills the platform at API\nrates. The one method a [`provider`](super::Agent::provider)\ncomposes with.",
          "type": "object",
          "properties": {
            "method": {
              "type": "string",
              "const": "api_key"
            }
          },
          "required": [
            "method"
          ]
        },
        {
          "description": "A personal access token (`at-\u00e2\u20ac\u00a6`) or an Agent Identity JWT.\nDeclares `CODEX_ACCESS_TOKEN`, `fixed`: a JSON string, put in\nthe process environment under the same name, where Codex's\ncredential store reads it after `CODEX_API_KEY`.",
          "type": "object",
          "properties": {
            "method": {
              "type": "string",
              "const": "access_token"
            }
          },
          "required": [
            "method"
          ]
        },
        {
          "description": "Codex's own `auth.json`, whole. Declares `CODEX_AUTH`,\n`rotating`: a JSON object of any kind the file takes \u00e2\u20ac\u201d ChatGPT\ntokens (`auth_mode`, `tokens {id_token, access_token,\nrefresh_token, account_id}`, `last_refresh`), an `OPENAI_API_KEY`\nmember, a `personal_access_token`, an `agent_identity`, Bedrock\ncredentials \u00e2\u20ac\u201d written to `$CODEX_HOME/auth.json` before every\nloop and read back after the loop's last turn. Codex rewrites\nthe file in place when a ChatGPT login refreshes \u00e2\u20ac\u201d the three\ntokens and `last_refresh` rotate, five minutes before the\naccess token's expiry, eight days after `last_refresh`, or on a\n`401` \u00e2\u20ac\u201d and what it leaves is what the collect answers, so the\nnext activation finds the refreshed login. Bills whatever the\nlogin is.",
          "type": "object",
          "properties": {
            "method": {
              "type": "string",
              "const": "auth_file"
            }
          },
          "required": [
            "method"
          ]
        },
        {
          "description": "Workload identity federation: an identity token exchanged\nagainst a federation rule. Declares `OPENAI_IDENTITY_TOKEN`,\n`fixed`: a JSON string, written to `$CODEX_HOME/identity-token`\nand named to Codex by `OPENAI_IDENTITY_TOKEN_FILE`, with\n`OPENAI_FEDERATION_RULE_ID` and, when given,\n`OPENAI_WORKLOAD_IDENTITY_CONTEXT` from the two members here.",
          "type": "object",
          "properties": {
            "federation_rule_id": {
              "description": "The federation rule the token is exchanged against:\n`OPENAI_FEDERATION_RULE_ID`.",
              "type": "string"
            },
            "context": {
              "description": "The workload's context, when the rule wants one:\n`OPENAI_WORKLOAD_IDENTITY_CONTEXT`. Absent leaves the\nvariable unset.",
              "type": [
                "string",
                "null"
              ]
            },
            "method": {
              "type": "string",
              "const": "workload_identity"
            }
          },
          "required": [
            "method",
            "federation_rule_id"
          ]
        }
      ]
    },
    "Template": {
      "description": "One tool container an agent's program depends on: everything the\ncaller needs to deploy it, as fields and not as words. A tool\ncontainer depends on nothing.\n\nA [`Container`](crate::shared::containers::request::Container) is an\nimage, two limits, mounts and arguments. The image, the limits and\nthe arguments are here, member for member; beside them the secrets\nthe image needs, by name alone \u00e2\u20ac\u201d which of them the tool's program\ntakes, and in what mode, is its own answer at registration, and\ntheir values are the caller's, each resolved through the vault key\nthe agent that spawns the tool binds the same name to, fed at every\nactivation. The mounts are not the caller's volumes \u00e2\u20ac\u201d a volume is\nnamed in the caller's own listing, and a tool that named one would\nbe dictating the caller's storage \u00e2\u20ac\u201d but, for a transient tool alone,\nthe AGENT's own files and directories, each a path in the agent\ncontainer served live at a path in the tool container: the caller\nserves the agent container, a `containers::fuse_serve` of the path,\nand mounts the tool, a `fuse::mount` answered from it.\n[`mode`](Self::mode) says which of the two modes the dependency is \u00e2\u20ac\u201d\na transient tool of the agent's run, with those mounts, or a service\nthe caller keeps, with a scope \u00e2\u20ac\u201d see [`Mode`]; and\n[`permissions`](Self::permissions) what its account may do \u00e2\u20ac\u201d grants\nthat name nothing, so the template is the same on every daemon it\nreaches, and among them the word `self`, which reaches the agent a\ntransient tool is the dependency of and every agent attached to a\nservice.\n\nThere are no instructions. What a template cannot say \u00e2\u20ac\u201d the provider\nthe tool runs on, the volumes it is given, the account's name \u00e2\u20ac\u201d is\nthe caller's, and the caller needs no prose to choose it. The\ndaemon's own tool templates are another thing: a record the daemon\nkeeps, named by a hash, made by a request; this is what an image\ndeclares and a caller deploys.\n\nThere is no name. A dependency is named by its ID: the lowercase\nhexadecimal SHA-256 of its [`canonical`](crate::shared::canonical)\nbytes \u00e2\u20ac\u201d this template as compact JSON, absent members omitted, the\nimage's `references` absent too, every object key sorted at every\ndepth, the `arguments` and the `secrets` included, the names sorted\nbytewise, the `mode` and\nits members among the keys \u00e2\u20ac\u201d sixty-four characters, which the caller\ncomputes over [`hashed`](Self::hashed). Two declarations with one id\nare one dependency, and a list that declares one twice is refused;\nthe same template deployed for the same agent, on whatever provider,\nis the same dependency, and the same service template declared under\nthe same scope key, by whatever agent, is the same service. The\ncaller labels the tool's MCP tools by the id when it merges lists,\nand the program knows which tool serves each by the keys under\n`_meta`, see [`shared::mcp`](crate::shared::mcp).",
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
          "description": "The arguments the tool container is registered with \u00e2\u20ac\u201d the\n`arguments` of the run request that makes it, verbatim."
        },
        "secrets": {
          "description": "The secrets the image needs, by name, in the order the image\nstates them: names only. Which of them the tool's program takes\nand in what mode is the program's answer at its registration \u00e2\u20ac\u201d\nsee [`secrets`](crate::shared::containers::secrets) \u00e2\u20ac\u201d and the\nvalue of each is resolved through the vault key the agent the\ntool is spawned for binds the same name to, fed at every\nactivation. Inside the hash, sorted bytewise there. A name is\nnot empty, no two are alike, and every one is a name the\nregistering agent's template declares; a template that says\notherwise is the registration's error. Absent, the image needs\nnone.",
          "type": "array",
          "items": {
            "type": "string"
          }
        },
        "permissions": {
          "description": "What the tool's account may do, as grants that name nothing: see\n[`Grant`]. The caller gives the tool an account holding exactly\nthese. Absent when empty, and then the tool may do nothing\nthrough the daemon.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Grant"
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
      "description": "The image, by its manifest digest and the references under which\nits bytes may be fetched \u00e2\u20ac\u201d what\n[`images::check`](crate::provider::endpoints::images::check::client::request::Frame)\nasks about, so a check that came back available names an image a\nrun can ask for, with nothing to translate between them.\n\n# The digest is the image\n\nA digest cannot be repointed at different content, so an image\nmeans the same bytes every time, wherever they come from. That is\nthe whole of what the wire asserts: the provider runs the image\nthe digest names and no other. The references are not the image:\nthe same bytes sit under different names on different registries,\nand an id that hashes an image hashes it with none \u00e2\u20ac\u201d see\n[`hashed`](Self::hashed).\n\n# Who supplies the bytes is the provider's\n\nA provider looks in what it holds first, by digest alone \u00e2\u20ac\u201d among\nwhich are the images the caller\n[built](crate::provider::endpoints::images::build) on it, and no\nother caller's. Failing that it may ask any referenced registry its own policy lists, with\nthe credential it holds for it, and ignores a reference naming one\nit does not; and it may take the image from the caller \u00e2\u20ac\u201d asking on\nthe run scope whether the caller holds it and, when it does, for\nits manifest and blobs, see [`oci`](crate::shared::containers::oci).\nWhich of those, in what order, is the provider's policy, and a\ncaller learns only whether the run happened. A caller need not\nhold what it names; one that does answers so when asked.",
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
      "description": "One place the bytes an image's digest names may be fetched from: a\nregistry, and the repository path there. A reference locates and\ndoes not identify \u00e2\u20ac\u201d the digest does \u00e2\u20ac\u201d so an image may carry none,\none or several, and two images with one digest are one image\nwhatever each lists. A provider asks a referenced registry only\nwhere its own policy lists that registry, with the credential it\nholds for it, and ignores a reference naming one it does not.",
      "type": "object",
      "properties": {
        "registry": {
          "description": "The registry's host, as a reference names it before its first\n`/`: `docker.io`, `ghcr.io`, `registry.example.com:5000`. No\nscheme and no path.",
          "type": "string"
        },
        "name": {
          "description": "The repository path at that registry \u00e2\u20ac\u201d `library/nginx`,\n`myorg/myimage`. No host and no tag: the image is\n`<registry>/<name>@<digest>` there.\n\nIt lands in a reference by concatenation wherever a provider\npulls from, so a provider refuses one that is not a repository\npath and normalizes nothing.",
          "type": "string"
        }
      },
      "required": [
        "registry",
        "name"
      ]
    },
    "Mount": {
      "description": "A path of the agent container, mounted at a path of the tool\ncontainer over FUSE: source before destination, the order a mount\nreads in everywhere else. Which of the two it is \u00e2\u20ac\u201d a file or a\ndirectory \u00e2\u20ac\u201d is which list of the [`Template`](super::Template) it\nis on.\n\nThe caller serves the agent container's path live \u00e2\u20ac\u201d a\n`containers::fuse_serve` of it, or of the directory holding a file \u00e2\u20ac\u201d\nand makes the mount in the tool container as a FUSE mount answered\nfrom that FUSE serve, so every read, write, listing, removal, rename and\nnew directory the tool makes is one ask answered from the agent's\nown filesystem, where the agent sees it at once. Nothing is copied:\nthe two containers share the path for as long as both run.",
      "type": "object",
      "properties": {
        "agent_path": {
          "description": "Where it is in the agent container, as path components from the\nagent container's root \u00e2\u20ac\u201d the shape every path in this crate\ntakes. No component is empty, `.` or `..`, and the path is not\nthe root: a file on the file list, a directory on the directory\nlist.",
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
      "description": "What a service tool is one of: one per parent agent, one per\nparent agent template, or one for every agent that declares it,\nwhatever template they are made from. The scope names the service\n\u00e2\u20ac\u201d the same template declared under the same scope key is the same\nservice, joined and not made again \u00e2\u20ac\u201d and names its database scope\nwith it, so what a service holds is shared exactly as far as the\nservice is. Snake case on the wire: `\"instance\"`,\n`\"template\"`, `\"global\"`.",
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
    },
    "Grant": {
      "description": "One grant that names nothing: a permission over one kind of thing\nthe daemon holds, reaching by tags alone. Externally tagged by the\nkind's name \u00e2\u20ac\u201d one object with one member, `{\"agents\":\u00e2\u20ac\u00a6}`,\n`{\"volumes\":\u00e2\u20ac\u00a6}` \u00e2\u20ac\u201d whose value is that kind's\n[`Permission`](agents::Permission), in one of the shapes\n[`permission`](crate::shared::permission) states. Snake case on the\nwire: `\"agents\"`, `\"agents_templates\"`, `\"standalone_tools\"`,\n`\"standalone_tools_templates\"`, `\"dependency_tools\"`,\n`\"providers_outgoing\"`,\n`\"providers_incoming\"`, `\"providers_daemons\"`, `\"accounts\"`,\n`\"roles\"`, `\"volumes\"`, `\"vault\"`, `\"postgres\"`, `\"events\"` \u00e2\u20ac\u201d the\nkinds of the daemon's [`Grant`](crate::daemon::grant::Grant), which\nis made of these.",
      "oneOf": [
        {
          "description": "Over agents: see [`agents::Permission`].",
          "type": "object",
          "properties": {
            "agents": {
              "$ref": "#/$defs/Permission"
            }
          },
          "required": [
            "agents"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over agent templates: see [`agents_templates::Permission`].",
          "type": "object",
          "properties": {
            "agents_templates": {
              "$ref": "#/$defs/Permission2"
            }
          },
          "required": [
            "agents_templates"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over standalone tools: see [`standalone_tools::Permission`].",
          "type": "object",
          "properties": {
            "standalone_tools": {
              "$ref": "#/$defs/Permission3"
            }
          },
          "required": [
            "standalone_tools"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over standalone tool templates: see\n[`standalone_tools_templates::Permission`].",
          "type": "object",
          "properties": {
            "standalone_tools_templates": {
              "$ref": "#/$defs/Permission4"
            }
          },
          "required": [
            "standalone_tools_templates"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over dependency tools: see [`dependency_tools::Permission`].",
          "type": "object",
          "properties": {
            "dependency_tools": {
              "$ref": "#/$defs/Permission5"
            }
          },
          "required": [
            "dependency_tools"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over outgoing providers: see [`providers_outgoing::Permission`].",
          "type": "object",
          "properties": {
            "providers_outgoing": {
              "$ref": "#/$defs/Permission6"
            }
          },
          "required": [
            "providers_outgoing"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over incoming credentials: see [`providers_incoming::Permission`].",
          "type": "object",
          "properties": {
            "providers_incoming": {
              "$ref": "#/$defs/Permission7"
            }
          },
          "required": [
            "providers_incoming"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over daemons: see [`providers_daemons::Permission`].",
          "type": "object",
          "properties": {
            "providers_daemons": {
              "$ref": "#/$defs/Permission8"
            }
          },
          "required": [
            "providers_daemons"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over accounts: see [`accounts::Permission`].",
          "type": "object",
          "properties": {
            "accounts": {
              "$ref": "#/$defs/Permission9"
            }
          },
          "required": [
            "accounts"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over roles: see [`roles::Permission`].",
          "type": "object",
          "properties": {
            "roles": {
              "$ref": "#/$defs/Permission10"
            }
          },
          "required": [
            "roles"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over volumes: see [`volumes::Permission`].",
          "type": "object",
          "properties": {
            "volumes": {
              "$ref": "#/$defs/Permission11"
            }
          },
          "required": [
            "volumes"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over the vault's keys: see [`vault::Permission`].",
          "type": "object",
          "properties": {
            "vault": {
              "$ref": "#/$defs/Permission12"
            }
          },
          "required": [
            "vault"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over the database: see [`postgres::Permission`].",
          "type": "object",
          "properties": {
            "postgres": {
              "$ref": "#/$defs/Permission13"
            }
          },
          "required": [
            "postgres"
          ],
          "additionalProperties": false
        },
        {
          "description": "Over the events: see [`events::Permission`].",
          "type": "object",
          "properties": {
            "events": {
              "$ref": "#/$defs/Permission14"
            }
          },
          "required": [
            "events"
          ],
          "additionalProperties": false
        }
      ]
    },
    "Permission": {
      "description": "What one grant over agents allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach \u00e2\u20ac\u201d every agent, the dependency's own, or by tags; or an object\nof tagging actions, how far they reach, and which tags. An object\nthat is none of these \u00e2\u20ac\u201d an action of one shape among another's, a\nmember missing, a `within` that is not `\"any\"`, `\"self\"` or a\n[`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make"
          }
        },
        {
          "description": "Over agents that exist: the [`Over`] actions held, reaching\nevery one of them, the dependency's own agents, or only those\nthe [`Tags`] let through, read as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, `\"self\"`, or the tags as a test. See\n[`Within`].",
              "$ref": "#/$defs/AgentsWithin"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging agents: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make": {
      "description": "The actions over agents that make one where there was none, which a\ngrant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Create an agent, as\n[`agents::create`](crate::daemon::endpoints::agents::create)\ndoes.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over": {
      "description": "The actions over agents that exist, which a grant reaches as far as\nits `within` says. Snake case on the wire: `\"get\"`, `\"delete\"`,\n`\"edit\"`, `\"message\"`, `\"logs\"`, `\"list\"`, `\"download\"`, `\"upload\"`,\n`\"transfer\"`, `\"filetree\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`agents::get`](crate::daemon::endpoints::agents::get) does.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "Delete one, as\n[`agents::delete`](crate::daemon::endpoints::agents::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Change one, as\n[`agents::edit`](crate::daemon::endpoints::agents::edit) does.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Send one a message, as\n[`agents::message`](crate::daemon::endpoints::agents::message)\ndoes.",
          "type": "string",
          "const": "message"
        },
        {
          "description": "Read one's log, as\n[`agents::logs`](crate::daemon::endpoints::agents::logs) does.",
          "type": "string",
          "const": "logs"
        },
        {
          "description": "List them, as\n[`agents::list`](crate::daemon::endpoints::agents::list) does;\nthe list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Send the client files out of one, as\n[`agents::download`](crate::daemon::endpoints::agents::download)\ndoes.",
          "type": "string",
          "const": "download"
        },
        {
          "description": "Put files into one, as\n[`agents::upload`](crate::daemon::endpoints::agents::upload)\ndoes, and as a transfer landing in one does.",
          "type": "string",
          "const": "upload"
        },
        {
          "description": "Copy files out of one, as\n[`agents::transfer`](crate::daemon::endpoints::agents::transfer)\ndoes; where they land is judged by its own grant.",
          "type": "string",
          "const": "transfer"
        },
        {
          "description": "Watch one's container whole, as\n[`agents::filetree`](crate::daemon::endpoints::agents::filetree)\ndoes.",
          "type": "string",
          "const": "filetree"
        }
      ]
    },
    "AgentsWithin": {
      "anyOf": [
        {
          "type": "string",
          "const": "any"
        },
        {
          "type": "string",
          "const": "self"
        },
        {
          "$ref": "#/$defs/Tags"
        }
      ]
    },
    "Tags": {
      "description": "Which of a kind a grant reaches, by their tags: those carrying\nevery one of `all_tags` and any one of `any_tags`. A member that\nis empty is absent, and passes everything; both empty is every one\nof the kind, which `\"any\"` says more plainly. Read as a TEST, as\nthe daemon reads a `within`: a thing passes when it passes every\nmember given. The two words are the ones every list filter of the\ndaemon's already has, so a grant here and a grant of the daemon's\nread tags the same way. No other member decodes: a `within` that\nnamed something \u00e2\u20ac\u201d `names`, `templates`, `creators` \u00e2\u20ac\u201d would\notherwise read as no tags at all, which is everything, and a grant\nmust never widen by being misread.",
      "type": "object",
      "properties": {
        "all_tags": {
          "description": "Every one of these among the thing's tags. Absent when empty,\nand then any tags.",
          "type": "array",
          "items": {
            "type": "string"
          }
        },
        "any_tags": {
          "description": "Any one of these among the thing's tags; with `all_tags`, both\nhold. Absent when empty, and then any tags.",
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      },
      "additionalProperties": false
    },
    "Tagging": {
      "description": "Putting tags on, or taking them off: the actions of a tagging grant,\nthe same two for every kind that carries tags. Snake case on the\nwire: `\"tag\"`, `\"untag\"`.",
      "oneOf": [
        {
          "description": "Put tags on, as the kind's `tag` endpoint does.",
          "type": "string",
          "const": "tag"
        },
        {
          "description": "Take tags off, as the kind's `untag` endpoint does.",
          "type": "string",
          "const": "untag"
        }
      ]
    },
    "Within_Tags": {
      "anyOf": [
        {
          "type": "string",
          "const": "any"
        },
        {
          "$ref": "#/$defs/Tags"
        }
      ]
    },
    "Within_Array_of_string": {
      "anyOf": [
        {
          "type": "string",
          "const": "any"
        },
        {
          "type": "array",
          "items": {
            "type": "string"
          }
        }
      ]
    },
    "Permission2": {
      "description": "What one grant over agent templates allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make2"
          }
        },
        {
          "description": "Over agent templates that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over2"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging agent templates: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make2": {
      "description": "The actions over agent templates that make one where there was none,\nwhich a grant holds or does not and judges by nothing else. Snake\ncase on the wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Make a template, as\n[`agents::templates::create`](crate::daemon::endpoints::agents::templates::create)\ndoes.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over2": {
      "description": "The actions over agent templates that exist, which a grant reaches\nas far as its `within` says. Snake case on the wire: `\"get\"`,\n`\"attach\"`, `\"delete\"`, `\"list\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`agents::templates::get`](crate::daemon::endpoints::agents::templates::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "Attach a tool to every agent of one, or detach one, as\n[`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template)\nand\n[`detach_agent_template`](crate::daemon::endpoints::standalone_tools::detach_agent_template)\ndo; the tool is judged by an `attach` or a `detach` grant over\ntools.",
          "type": "string",
          "const": "attach"
        },
        {
          "description": "Delete one, as\n[`agents::templates::delete`](crate::daemon::endpoints::agents::templates::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "List them, as\n[`agents::templates::list`](crate::daemon::endpoints::agents::templates::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        }
      ]
    },
    "Permission3": {
      "description": "What one grant over tools allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make3"
          }
        },
        {
          "description": "Over tools that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over3"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging tools: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make3": {
      "description": "The actions over tools that make one where there was none, which a\ngrant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"create\"`, `\"register\"`.",
      "oneOf": [
        {
          "description": "Create a tool, as\n[`tools::create`](crate::daemon::endpoints::standalone_tools::create) does.",
          "type": "string",
          "const": "create"
        },
        {
          "description": "Hold another daemon's tool under a name, as\n[`tools::register`](crate::daemon::endpoints::standalone_tools::register)\ndoes.",
          "type": "string",
          "const": "register"
        }
      ]
    },
    "Over3": {
      "description": "The actions over tools that exist, which a grant reaches as far as\nits `within` says. Snake case on the wire: `\"get\"`, `\"edit\"`,\n`\"attach\"`, `\"detach\"`, `\"delete\"`, `\"list\"`, `\"download\"`,\n`\"upload\"`, `\"transfer\"`, `\"filetree\"`, `\"connect\"`.",
      "oneOf": [
        {
          "description": "Get one, as [`tools::get`](crate::daemon::endpoints::standalone_tools::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "Change one, as\n[`tools::edit`](crate::daemon::endpoints::standalone_tools::edit) does.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Attach one to an agent, as\n[`attach_agent_instance`](crate::daemon::endpoints::standalone_tools::attach_agent_instance)\ndoes, the agent judged by an `edit` grant over agents; or to\nevery agent of a template, as\n[`attach_agent_template`](crate::daemon::endpoints::standalone_tools::attach_agent_template)\ndoes, the template judged by an `attach` grant over agent\ntemplates.",
          "type": "string",
          "const": "attach"
        },
        {
          "description": "Detach one from an agent, as\n[`detach_agent_instance`](crate::daemon::endpoints::standalone_tools::detach_agent_instance)\ndoes, the agent judged by an `edit` grant over agents; or from\nevery agent of a template, as\n[`detach_agent_template`](crate::daemon::endpoints::standalone_tools::detach_agent_template)\ndoes, the template judged by an `attach` grant over agent\ntemplates.",
          "type": "string",
          "const": "detach"
        },
        {
          "description": "Delete one, as\n[`tools::delete`](crate::daemon::endpoints::standalone_tools::delete) does.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "List them, as\n[`tools::list`](crate::daemon::endpoints::standalone_tools::list) does; the\nlist sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Send the client files out of one, as\n[`tools::download`](crate::daemon::endpoints::standalone_tools::download)\ndoes.",
          "type": "string",
          "const": "download"
        },
        {
          "description": "Put files into one, as\n[`tools::upload`](crate::daemon::endpoints::standalone_tools::upload) does,\nand as a transfer landing in one does.",
          "type": "string",
          "const": "upload"
        },
        {
          "description": "Copy files out of one, as\n[`tools::transfer`](crate::daemon::endpoints::standalone_tools::transfer)\ndoes; where they land is judged by its own grant.",
          "type": "string",
          "const": "transfer"
        },
        {
          "description": "Watch one's container whole, as\n[`tools::filetree`](crate::daemon::endpoints::standalone_tools::filetree)\ndoes.",
          "type": "string",
          "const": "filetree"
        },
        {
          "description": "Serve one to another daemon, as\n[`tools::connect`](crate::daemon::endpoints::standalone_tools::connect) does.",
          "type": "string",
          "const": "connect"
        }
      ]
    },
    "Permission4": {
      "description": "What one grant over tool templates allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make4"
          }
        },
        {
          "description": "Over tool templates that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over4"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging tool templates: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make4": {
      "description": "The actions over tool templates that make one where there was none,\nwhich a grant holds or does not and judges by nothing else. Snake\ncase on the wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Make a tool template, as\n[`tools::templates::create`](crate::daemon::endpoints::standalone_tools::templates::create)\ndoes.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over4": {
      "description": "The actions over tool templates that exist, which a grant reaches as\nfar as its `within` says. Snake case on the wire: `\"get\"`,\n`\"delete\"`, `\"list\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`tools::templates::get`](crate::daemon::endpoints::standalone_tools::templates::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "Delete one, as\n[`tools::templates::delete`](crate::daemon::endpoints::standalone_tools::templates::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "List them, as\n[`tools::templates::list`](crate::daemon::endpoints::standalone_tools::templates::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        }
      ]
    },
    "Permission5": {
      "description": "What one grant over dependency templates allows, in one of two\nshapes told apart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: an object\nof [`Over`] actions and how far they reach, by tags; or an object\nof tagging actions, how far they reach, and which tags. There is\nno making shape: nothing on the wire makes a dependency template.\nAn object that is neither \u00e2\u20ac\u201d an action of one shape among the\nother's, a member missing, a `within` that is not `\"any\"` or a\n[`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "Over dependency templates that exist: the [`Over`] actions\nheld, reaching every one of them, or only those the [`Tags`]\nlet through, read as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over5"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging dependency templates: putting tags on, taking them off,\nor both, reaching every one or only those the [`Tags`] let\nthrough, and touching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Over5": {
      "description": "The actions over dependency tools that exist, which a grant\nreaches as far as its `within` says: each tool is judged on its\nown, by the tools list filter; a delete, which is by template,\ntakes the action over every tool of the template.\nSnake case on the wire: `\"get\"`, `\"delete\"`, `\"list\"`,\n`\"download\"`, `\"upload\"`, `\"transfer\"`, `\"filetree\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`dependencies::get`](crate::daemon::endpoints::dependency_tools::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "Delete every tool of a template, as\n[`dependencies::delete`](crate::daemon::endpoints::dependency_tools::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "List them, as\n[`dependencies::list`](crate::daemon::endpoints::dependency_tools::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Send the client files out of one, running, as\n[`dependencies::download`](crate::daemon::endpoints::dependency_tools::download)\ndoes.",
          "type": "string",
          "const": "download"
        },
        {
          "description": "Put files into one, running, as\n[`dependencies::upload`](crate::daemon::endpoints::dependency_tools::upload)\ndoes, and as a transfer landing in one does.",
          "type": "string",
          "const": "upload"
        },
        {
          "description": "Copy files out of one, running, as\n[`dependencies::transfer`](crate::daemon::endpoints::dependency_tools::transfer)\ndoes; where they land is judged by its own grant.",
          "type": "string",
          "const": "transfer"
        },
        {
          "description": "Watch one's container whole, as\n[`dependencies::filetree`](crate::daemon::endpoints::dependency_tools::filetree)\ndoes.",
          "type": "string",
          "const": "filetree"
        }
      ]
    },
    "Permission6": {
      "description": "What one grant over outgoing providers allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make5"
          }
        },
        {
          "description": "Over outgoing providers that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over6"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging outgoing providers: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make5": {
      "description": "The actions over outgoing providers that make one where there was\nnone, which a grant holds or does not and judges by nothing else.\nSnake case on the wire: `\"add\"`.",
      "oneOf": [
        {
          "description": "Add a provider to dial, as\n[`providers::outgoing::add`](crate::daemon::endpoints::providers::outgoing::add)\ndoes.",
          "type": "string",
          "const": "add"
        }
      ]
    },
    "Over6": {
      "description": "The actions over outgoing providers that exist, which a grant\nreaches as far as its `within` says. Snake case on the wire:\n`\"get\"`, `\"list\"`, `\"delete\"`, `\"edit\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`providers::outgoing::get`](crate::daemon::endpoints::providers::outgoing::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`providers::outgoing::list`](crate::daemon::endpoints::providers::outgoing::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Forget one, as\n[`providers::outgoing::delete`](crate::daemon::endpoints::providers::outgoing::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Replace one's mode, as\n[`providers::outgoing::edit`](crate::daemon::endpoints::providers::outgoing::edit)\ndoes.",
          "type": "string",
          "const": "edit"
        }
      ]
    },
    "Permission7": {
      "description": "What one grant over incoming credentials allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make6"
          }
        },
        {
          "description": "Over incoming credentials that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over7"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging incoming credentials: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make6": {
      "description": "The actions over incoming credentials that make one where there was\nnone, which a grant holds or does not and incoming credentials by\nnothing else. Snake case on the wire: `\"add\"`.",
      "oneOf": [
        {
          "description": "Add an incoming credential of incoming providers, as\n[`providers::incoming::add`](crate::daemon::endpoints::providers::incoming::add)\ndoes.",
          "type": "string",
          "const": "add"
        }
      ]
    },
    "Over7": {
      "description": "The actions over incoming credentials that exist, which a grant\nreaches as far as its `within` says. Snake case on the wire:\n`\"get\"`, `\"list\"`, `\"delete\"`, `\"edit\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`providers::incoming::get`](crate::daemon::endpoints::providers::incoming::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`providers::incoming::list`](crate::daemon::endpoints::providers::incoming::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Take one out, as\n[`providers::incoming::delete`](crate::daemon::endpoints::providers::incoming::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Replace one, as\n[`providers::incoming::edit`](crate::daemon::endpoints::providers::incoming::edit)\ndoes.",
          "type": "string",
          "const": "edit"
        }
      ]
    },
    "Permission8": {
      "description": "What one grant over daemons allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions, how far they\nreach, by tags, and which of the records' accounts the two actions\ndone as an account \u00e2\u20ac\u201d `connect`, `relay` \u00e2\u20ac\u201d may act as; or an object\nof tagging actions, how far they reach, and which tags. An object\nthat is none of these \u00e2\u20ac\u201d an action of one shape among another's, a\nmember missing, a `within` that is not `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does\nnot decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make7"
          }
        },
        {
          "description": "Over daemons that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through, read\nas a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over8"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "accounts": {
              "description": "Which accounts, for `connect` and `relay`: `\"any\"`, or only\nthe account names listed, on whichever record `within`\nreaches. Absent is `\"any\"`. The other actions are over the\nrecord and read no account.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging daemons: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make7": {
      "description": "The actions over daemons that make one where there was none, which a\ngrant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"add\"`.",
      "oneOf": [
        {
          "description": "Add a daemon to connect to, as\n[`providers::daemons::add`](crate::daemon::endpoints::providers::daemons::add)\ndoes.",
          "type": "string",
          "const": "add"
        }
      ]
    },
    "Over8": {
      "description": "The actions over daemons that exist, which a grant reaches as far as\nits `within` says \u00e2\u20ac\u201d and, for the two done AS one of the record's\naccounts, as far as its `accounts` says. Snake case on the wire:\n`\"get\"`, `\"list\"`, `\"delete\"`, `\"edit\"`, `\"connect\"`, `\"relay\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`providers::daemons::get`](crate::daemon::endpoints::providers::daemons::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`providers::daemons::list`](crate::daemon::endpoints::providers::daemons::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Forget one, as\n[`providers::daemons::delete`](crate::daemon::endpoints::providers::daemons::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Replace one's links, as\n[`providers::daemons::edit`](crate::daemon::endpoints::providers::daemons::edit)\ndoes; and add, rotate and forget its accounts, as\n[`providers::daemons::accounts`](crate::daemon::endpoints::providers::daemons::accounts)\ndoes.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Reach a tool of the daemon's as one of the record's accounts,\nas\n[`standalone_tools::register`](crate::daemon::endpoints::standalone_tools::register)\nnames one and the connected tool's use opens it. Judged against\nthe grant's `accounts` as well as its `within`.",
          "type": "string",
          "const": "connect"
        },
        {
          "description": "Put a request to the daemon as one of the record's accounts \u00e2\u20ac\u201d\nany request of the wire carrying a `relay` naming the record\nand the account. Judged against the grant's `accounts` as well\nas its `within`.",
          "type": "string",
          "const": "relay"
        }
      ]
    },
    "Permission9": {
      "description": "What one grant over accounts allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make8"
          }
        },
        {
          "description": "Over accounts that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over9"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging accounts: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make8": {
      "description": "The actions over accounts that make one where there was none, which\na grant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Create an account, as\n[`accounts::create`](crate::daemon::endpoints::accounts::create)\ndoes.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over9": {
      "description": "The actions over accounts that exist, which a grant reaches as far\nas its `within` says. Snake case on the wire: `\"get\"`, `\"list\"`,\n`\"delete\"`, `\"edit\"`, `\"assign\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`accounts::get`](crate::daemon::endpoints::accounts::get) does.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`accounts::list`](crate::daemon::endpoints::accounts::list)\ndoes; the list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Delete one, as\n[`accounts::delete`](crate::daemon::endpoints::accounts::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Change one, as\n[`accounts::edit`](crate::daemon::endpoints::accounts::edit)\ndoes.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Name one as a container's `account`, at an\n[agent's](crate::daemon::endpoints::agents::create) or a\n[tool's](crate::daemon::endpoints::standalone_tools::create) create or\nedit.",
          "type": "string",
          "const": "assign"
        }
      ]
    },
    "Permission10": {
      "description": "What one grant over roles allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make9"
          }
        },
        {
          "description": "Over roles that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over10"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging roles: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make9": {
      "description": "The actions over roles that make one where there was none, which a\ngrant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Create a role, as\n[`roles::create`](crate::daemon::endpoints::roles::create) does.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over10": {
      "description": "The actions over roles that exist, which a grant reaches as far as\nits `within` says. Snake case on the wire: `\"get\"`, `\"list\"`,\n`\"delete\"`, `\"edit\"`, `\"grant\"`.",
      "oneOf": [
        {
          "description": "Get one, as [`roles::get`](crate::daemon::endpoints::roles::get)\ndoes.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`roles::list`](crate::daemon::endpoints::roles::list) does; the\nlist sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Delete one, as\n[`roles::delete`](crate::daemon::endpoints::roles::delete) does.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Change one, as\n[`roles::edit`](crate::daemon::endpoints::roles::edit) does.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Name one in an account's `roles`, at the account's\n[create](crate::daemon::endpoints::accounts::create) or\n[edit](crate::daemon::endpoints::accounts::edit).",
          "type": "string",
          "const": "grant"
        }
      ]
    },
    "Permission11": {
      "description": "What one grant over volumes allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make10"
          }
        },
        {
          "description": "Over volumes that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over11"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging volumes: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make10": {
      "description": "The actions over volumes that make one where there was none, which a\ngrant holds or does not and judges by nothing else. Snake case on\nthe wire: `\"create\"`.",
      "oneOf": [
        {
          "description": "Create a volume on a provider, as\n[`volumes::create`](crate::daemon::endpoints::volumes::create)\ndoes.",
          "type": "string",
          "const": "create"
        }
      ]
    },
    "Over11": {
      "description": "The actions over volumes that exist, which a grant reaches as far as\nits `within` says. Snake case on the wire: `\"get\"`, `\"list\"`,\n`\"delete\"`, `\"edit\"`, `\"stat\"`, `\"download\"`, `\"upload\"`,\n`\"transfer\"`, `\"filetree\"`, `\"mount\"`.",
      "oneOf": [
        {
          "description": "Get one, as\n[`volumes::get`](crate::daemon::endpoints::volumes::get) does.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`volumes::list`](crate::daemon::endpoints::volumes::list) does;\nthe list sends what the grant reaches.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Delete one, as\n[`volumes::delete`](crate::daemon::endpoints::volumes::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Change one's size or mode, as\n[`volumes::edit`](crate::daemon::endpoints::volumes::edit) does.",
          "type": "string",
          "const": "edit"
        },
        {
          "description": "Walk one, as\n[`volumes::stat`](crate::daemon::endpoints::volumes::stat) does.",
          "type": "string",
          "const": "stat"
        },
        {
          "description": "Send the client files out of one, as\n[`volumes::download`](crate::daemon::endpoints::volumes::download)\ndoes.",
          "type": "string",
          "const": "download"
        },
        {
          "description": "Put files into one, as\n[`volumes::upload`](crate::daemon::endpoints::volumes::upload)\ndoes, and as a transfer landing in one does.",
          "type": "string",
          "const": "upload"
        },
        {
          "description": "Copy files out of one, as\n[`volumes::transfer`](crate::daemon::endpoints::volumes::transfer)\ndoes; where they land is judged by its own grant.",
          "type": "string",
          "const": "transfer"
        },
        {
          "description": "Name one in a container's mounts \u00e2\u20ac\u201d a volume mount of the\nprovider it is pinned to, or a FUSE mount of a file or a\ndirectory in it \u00e2\u20ac\u201d at the container's\n[create](crate::daemon::endpoints::agents::create) or\n[edit](crate::daemon::endpoints::agents::edit), beside the grant\nover the container.",
          "type": "string",
          "const": "mount"
        },
        {
          "description": "See one's tree, as\n[`volumes::filetree`](crate::daemon::endpoints::volumes::filetree)\ndoes.",
          "type": "string",
          "const": "filetree"
        }
      ]
    },
    "Permission12": {
      "description": "What one grant over the vault's keys allows, in one of three shapes told\napart on the wire by their very form, as\n[`permission`](crate::shared::permission) states them: a bare array\nof [`Make`] actions; an object of [`Over`] actions and how far they\nreach, by tags; or an object of tagging actions, how far they\nreach, and which tags. An object that is none of these \u00e2\u20ac\u201d an action\nof one shape among another's, a member missing, a `within` that is\nnot `\"any\"` or a [`Tags`] \u00e2\u20ac\u201d does not decode.",
      "anyOf": [
        {
          "description": "To make: the [`Make`] actions held, as a bare array. Nothing is\njudged but that the action is held.",
          "type": "array",
          "items": {
            "$ref": "#/$defs/Make11"
          }
        },
        {
          "description": "Over keys that exist: the [`Over`] actions held, reaching\nevery one of them, or only those the [`Tags`] let through,\nread as a test.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "The actions.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Over12"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within"
          ]
        },
        {
          "description": "Tagging keys: putting tags on, taking them off, or both,\nreaching every one or only those the [`Tags`] let through, and\ntouching any tag or only the ones listed.",
          "type": "object",
          "properties": {
            "actions": {
              "description": "`tag`, `untag`, or both.",
              "type": "array",
              "items": {
                "$ref": "#/$defs/Tagging"
              }
            },
            "within": {
              "description": "How far: `\"any\"`, or the tags as a test.",
              "$ref": "#/$defs/Within_Tags"
            },
            "tags": {
              "description": "Which tags: `\"any\"`, or only these.",
              "$ref": "#/$defs/Within_Array_of_string"
            }
          },
          "additionalProperties": false,
          "required": [
            "actions",
            "within",
            "tags"
          ]
        }
      ]
    },
    "Make11": {
      "description": "The actions over the vault that make a key where there was none,\nwhich a grant holds or does not and judges by nothing else. Snake\ncase on the wire: `\"insert\"`.",
      "oneOf": [
        {
          "description": "Insert into a key the vault does not hold, as\n[`vault::insert`](crate::daemon::endpoints::vault::insert) does\nwhere the key is new; over a key it holds, the same request is\n[`Over::Insert`](super::Over::Insert).",
          "type": "string",
          "const": "insert"
        }
      ]
    },
    "Over12": {
      "description": "The actions over vault keys that exist, which a grant reaches as\nfar as its `within` says. Snake case on the wire: `\"get\"`,\n`\"list\"`, `\"insert\"`, `\"delete\"`, `\"use\"`.",
      "oneOf": [
        {
          "description": "Get one, with its value, as\n[`vault::get`](crate::daemon::endpoints::vault::get) does.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List them, as\n[`vault::list`](crate::daemon::endpoints::vault::list) does;\nthe list sends what the grant reaches, and never a value.",
          "type": "string",
          "const": "list"
        },
        {
          "description": "Append a value to one, or replace one of its values, as\n[`vault::insert`](crate::daemon::endpoints::vault::insert) does\nover a key the vault holds.",
          "type": "string",
          "const": "insert"
        },
        {
          "description": "Delete one, or one of its values, as\n[`vault::delete`](crate::daemon::endpoints::vault::delete)\ndoes.",
          "type": "string",
          "const": "delete"
        },
        {
          "description": "Bind one to a secret of an agent's or a tool's, in the\n`secrets` of its [create](crate::daemon::create::Inner::secrets)\nor edit. The key must be one the vault holds, with at least\none value: a bound key is read or locked at every activation,\nand one with nothing to give is refused at the binding.",
          "type": "string",
          "const": "use"
        }
      ]
    },
    "Permission13": {
      "description": "The actions held over the database, a bare array on the wire:\n`[\"get\",\"connections\"]`. There is one database, so there is no\n`within`; an action is held or it is not.",
      "type": "array",
      "items": {
        "$ref": "#/$defs/Action"
      }
    },
    "Action": {
      "description": "The actions over the database, each the endpoint of the same name\nunder [`postgres`](crate::daemon::endpoints::postgres).",
      "oneOf": [
        {
          "description": "Read which database the daemon serves, a remote URL without its\npassword.",
          "type": "string",
          "const": "get"
        },
        {
          "description": "List the container connections open through it.",
          "type": "string",
          "const": "list"
        }
      ]
    },
    "Permission14": {
      "description": "The actions held over the events, a bare array on the wire:\n`[\"list\"]`. There is one graph of events, so there is no `within`;\nan action is held or it is not.",
      "type": "array",
      "items": {
        "$ref": "#/$defs/Action2"
      }
    },
    "Action2": {
      "description": "The actions over the events, each the endpoint of the same name\nunder [`events`](crate::daemon::endpoints::events).",
      "oneOf": [
        {
          "description": "List the events, narrowed, and keep the list.",
          "type": "string",
          "const": "list"
        }
      ]
    }
  }
}
---
# Codex

An agent container running an OpenAI Codex CLI agent, pre-installed
in the image, one `codex exec` per turn on a resumed thread.

Secrets: one, chosen by `auth.method` — `api_key` declares
`OPENAI_API_KEY` (fixed); `access_token` declares `CODEX_ACCESS_TOKEN`
(fixed); `auth_file` declares `CODEX_AUTH` (rotating, the whole
`auth.json`); `workload_identity` declares `OPENAI_IDENTITY_TOKEN`
(fixed).
