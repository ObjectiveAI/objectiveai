/**
 * The `diverge` plugin: the caller's MCP tools and resources, through
 * the proxy, as native Eliza actions and one provider. Built before the
 * runtime is constructed, inside `POST /run`, from the proxy's MCP
 * server at the URL the harness hands over — the one place this
 * project dials the proxy.
 *
 * The lists are LIVE. The proxy relays the caller's `tools/list_changed`
 * and `resources/list_changed` notifications, and this plugin listens
 * for both: a tool change re-lists and diffs the registered actions
 * (removed tools unregistered, new ones registered, changed ones
 * replaced under the same action name); a resource change re-lists the
 * cache the `DIVERGE_RESOURCES` provider renders into every turn. A
 * change lands on the next model call, not the one in flight. A
 * re-list that fails keeps the previous set and says so.
 *
 * Every call and every read is reported on the harness's line protocol
 * by this plugin itself, byte-faithfully — the MCP result verbatim, a
 * read's contents as embedded-resource blocks — and the runtime's own
 * tool envelopes for these actions are dropped by the entry, so each is
 * said exactly once, from the side that has the bytes.
 *
 * What the MODEL reads is `ActionResult.text`, and only that: the
 * planner stringifies the whole result as text, so nothing else goes on
 * it (a base64 block in `data` would land in the model's context as a
 * giant string). Each content block is rendered the way Eliza's own
 * ingress renders an inbound attachment: text as-is; an image described
 * through the cached `IMAGE_DESCRIPTION` tier with core's own prompt;
 * audio transcribed through `TRANSCRIPTION`; a blob by its type — text
 * and JSON decoded, a PDF read through `unpdf` as core reads one, an
 * image or audio as above, anything else named by uri, type and size.
 * A transcoder that fails says so in place, and the run goes on.
 */

import { Buffer } from "node:buffer";
import { randomUUID } from "node:crypto";

import {
  ModelType,
  describeImageCached,
  imageDescriptionTemplate,
  resolveOptimizedPromptForRuntime,
} from "@elizaos/core";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import {
  ResourceListChangedNotificationSchema,
  ToolListChangedNotificationSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { extractText } from "unpdf";

/** The action-name pattern the runtime enforces. */
const ACTION_NAME = /^[A-Z_][A-Z0-9_]*$/;

/** The resource read, and the provider that lists what it can read. */
const READ_RESOURCE = "DIVERGE_READ_RESOURCE";
const RESOURCES_PROVIDER = "DIVERGE_RESOURCES";

/**
 * Connect to the proxy's MCP server, list every tool and resource it
 * serves, subscribe to both list-changed notifications, and return the
 * plugin with `flush`, which the entry calls once the runtime is
 * initialized to apply any change that arrived before then. `emit`
 * takes one protocol line (an object) and writes it to the harness.
 */
export async function createDivergePlugin({ url, emit }) {
  const client = new Client({
    name: "diverge-agentic-loop-eliza",
    version: "2.3.0",
  });
  await client.connect(new StreamableHTTPClientTransport(new URL(url)));

  /** Action names in use; the read is reserved before any tool folds. */
  const taken = new Set([READ_RESOURCE]);
  /** MCP tool name → { actionName, action, signature }. */
  const tools = new Map();
  for (const tool of await listTools(client)) {
    tools.set(tool.name, entry(client, emit, tool, taken));
  }
  /** The resources, as last listed. */
  let resources = await listResources(client);

  /** The runtime, once `init` has run; refreshes wait for it. */
  let runtime = null;
  const dirty = { tools: false, resources: false };
  /** One refresh at a time per list, in notification order. */
  const chains = { tools: Promise.resolve(), resources: Promise.resolve() };

  async function refreshTools() {
    const listed = await listTools(client);
    const seen = new Set();
    const added = [];
    const removed = [];
    const changed = [];
    for (const tool of listed) {
      seen.add(tool.name);
      const existing = tools.get(tool.name);
      if (!existing) {
        const made = entry(client, emit, tool, taken);
        tools.set(tool.name, made);
        runtime.registerAction(made.action);
        added.push(tool.name);
      } else if (existing.signature !== signature(tool)) {
        runtime.unregisterAction(existing.actionName);
        const made = entry(client, emit, tool, taken, existing.actionName);
        tools.set(tool.name, made);
        runtime.registerAction(made.action);
        changed.push(tool.name);
      }
    }
    for (const [name, existing] of [...tools]) {
      if (!seen.has(name)) {
        runtime.unregisterAction(existing.actionName);
        taken.delete(existing.actionName);
        tools.delete(name);
        removed.push(name);
      }
    }
    if (added.length || removed.length || changed.length) {
      emit({
        type: "notification",
        message: { kind: "tools_changed", added, removed, changed },
      });
    }
  }

  async function refreshResources() {
    const listed = await listResources(client);
    const before = new Set(resources.map((resource) => resource.uri));
    const after = new Set(listed.map((resource) => resource.uri));
    const added = listed.map((r) => r.uri).filter((uri) => !before.has(uri));
    const removed = resources.map((r) => r.uri).filter((uri) => !after.has(uri));
    resources = listed;
    if (added.length || removed.length) {
      emit({
        type: "notification",
        message: { kind: "resources_changed", added, removed },
      });
    }
  }

  /**
   * One refresh of one list, queued behind the last: before the
   * runtime exists it only marks the list dirty, for `flush`.
   */
  function refresh(kind) {
    if (!runtime) {
      dirty[kind] = true;
      return chains[kind];
    }
    dirty[kind] = false;
    const run = kind === "tools" ? refreshTools : refreshResources;
    chains[kind] = chains[kind].then(run).catch((error) => {
      emit({
        type: "notification",
        message: { kind: "mcp", list: kind, error: describe(error) },
      });
    });
    return chains[kind];
  }

  client.setNotificationHandler(ToolListChangedNotificationSchema, () => {
    void refresh("tools");
  });
  client.setNotificationHandler(ResourceListChangedNotificationSchema, () => {
    void refresh("resources");
  });

  const plugin = {
    name: "diverge",
    description:
      "The caller's MCP tools and resources, through the Diverge container proxy",
    actions: [
      ...[...tools.values()].map((made) => made.action),
      readResourceAction(client, emit),
    ],
    providers: [resourcesProvider(() => resources)],
    init: async (_config, agentRuntime) => {
      runtime = agentRuntime;
    },
  };

  /** Apply what changed before the runtime was ready. */
  async function flush() {
    if (dirty.tools) await refresh("tools");
    if (dirty.resources) await refresh("resources");
  }

  return { plugin, flush };
}

/** Every tool the server lists, across pages. */
async function listTools(client) {
  const tools = [];
  let cursor;
  do {
    const page = await client.listTools(cursor ? { cursor } : undefined);
    tools.push(...(page.tools ?? []));
    cursor = page.nextCursor;
  } while (cursor);
  return tools;
}

/** Every resource the server lists, across pages. */
async function listResources(client) {
  const resources = [];
  let cursor;
  do {
    const page = await client.listResources(cursor ? { cursor } : undefined);
    resources.push(...(page.resources ?? []));
    cursor = page.nextCursor;
  } while (cursor);
  return resources;
}

/** What a tool's action is built from: a change here is a new action. */
function signature(tool) {
  return JSON.stringify({
    description: tool.description ?? null,
    inputSchema: tool.inputSchema ?? null,
  });
}

/** One tool's map entry: its action, under a kept or a new name. */
function entry(client, emit, tool, taken, keep) {
  const name = keep ?? actionName(tool.name, taken);
  return {
    actionName: name,
    action: action(client, emit, tool, name),
    signature: signature(tool),
  };
}

/** One tool as one action. */
function action(client, emit, tool, name) {
  return {
    name,
    description: tool.description ?? tool.name,
    parameters: parameters(tool.inputSchema),
    validate: async () => true,
    handler: async (runtime, _message, _state, options) => {
      const args = options?.parameters ?? {};
      const id = randomUUID();
      emit({
        type: "tool_call",
        id,
        name: tool.name,
        arguments: JSON.stringify(args),
      });
      let result;
      try {
        result = await client.callTool({ name: tool.name, arguments: args });
      } catch (error) {
        // The call never returned a result: the failure IS the result,
        // as the tool's caller would see it.
        result = {
          content: [{ type: "text", text: describe(error) }],
          isError: true,
        };
      }
      emit({ type: "tool_result", id, result });
      return {
        success: !result.isError,
        text: await render(runtime, result.content ?? []),
      };
    },
  };
}

/**
 * The resource read: one parameter, the URI as the provider listed
 * it. Reported as a tool call named `read_resource`, its result the
 * contents verbatim as embedded-resource blocks, and rendered to the
 * model exactly as those blocks would be in a tool's result.
 */
function readResourceAction(client, emit) {
  return {
    name: READ_RESOURCE,
    description:
      `Read one of the caller's resources by its URI. The resources available are listed under ${RESOURCES_PROVIDER}.`,
    parameters: [
      {
        name: "uri",
        description: "The resource's URI, exactly as listed.",
        required: true,
        schema: { type: "string" },
      },
    ],
    validate: async () => true,
    handler: async (runtime, _message, _state, options) => {
      const uri = String(options?.parameters?.uri ?? "");
      const id = randomUUID();
      emit({
        type: "tool_call",
        id,
        name: "read_resource",
        arguments: JSON.stringify({ uri }),
      });
      let result;
      try {
        const read = await client.readResource({ uri });
        result = {
          content: (read.contents ?? []).map((resource) => ({
            type: "resource",
            resource,
          })),
          isError: false,
        };
      } catch (error) {
        result = {
          content: [{ type: "text", text: describe(error) }],
          isError: true,
        };
      }
      emit({ type: "tool_result", id, result });
      return {
        success: !result.isError,
        text: await render(runtime, result.content),
      };
    },
  };
}

/** Every block, rendered for the model, one after another. */
async function render(runtime, blocks) {
  const parts = [];
  for (const block of blocks) {
    parts.push(await renderBlock(runtime, block));
  }
  return parts.join("\n");
}

/** One content block, as Eliza's ingress would render it. */
async function renderBlock(runtime, block) {
  switch (block.type) {
    case "text":
      return block.text ?? "";
    case "image":
      return describeImage(runtime, block.mimeType ?? "", block.data ?? "", "image");
    case "audio":
      return transcribe(runtime, block.mimeType ?? "", block.data ?? "", "audio");
    case "resource":
      return renderResource(runtime, block.resource ?? {});
    case "resource_link": {
      const name = block.title ?? block.name ?? block.uri;
      const type = block.mimeType ? ` (${block.mimeType})` : "";
      return `Resource link ${block.uri} — ${name}${type}; readable with ${READ_RESOURCE}`;
    }
    default:
      return `[${block.type ?? "unknown"} content]`;
  }
}

/** An embedded resource, or one entry of a read's contents. */
async function renderResource(runtime, resource) {
  const uri = resource.uri ?? "";
  const mime = resource.mimeType ?? "";
  const head = `Resource ${uri}${mime ? ` (${mime})` : ""}:`;
  if (typeof resource.text === "string") {
    return `${head}\n${resource.text}`;
  }
  if (typeof resource.blob !== "string") {
    return `${head} no contents`;
  }
  return `${head}\n${await renderBlob(runtime, uri, mime, resource.blob)}`;
}

/** A blob, by its type — core's ingress rules for an attachment. */
async function renderBlob(runtime, uri, mime, blob) {
  const kind = mime.toLowerCase();
  if (kind.startsWith("image/")) {
    return describeImage(runtime, mime, blob, `binary ${uri}`);
  }
  if (kind.startsWith("audio/")) {
    return transcribe(runtime, mime, blob, `binary ${uri}`);
  }
  const bytes = Buffer.from(blob, "base64");
  if (kind.startsWith("text/") || kind === "application/json") {
    return bytes.toString("utf8");
  }
  if (kind === "application/pdf") {
    try {
      const { text } = await extractText(new Uint8Array(bytes), { mergePages: true });
      return text;
    } catch (error) {
      return unread(`binary ${uri}`, mime, bytes.length, error);
    }
  }
  return `[binary ${uri}, ${mime || "unknown type"}, ${bytes.length} bytes]`;
}

/**
 * An image, described through core's content-addressed cache over
 * the `IMAGE_DESCRIPTION` tier, with the prompt core resolves for its
 * own inbound images — or, without that tier, named by type and size.
 */
async function describeImage(runtime, mime, data, label) {
  const size = Buffer.byteLength(data, "base64");
  if (typeof runtime.getModel(ModelType.IMAGE_DESCRIPTION) !== "function") {
    return `[${label} ${mime}, ${size} bytes; no image-description model is registered]`;
  }
  try {
    const prompt = resolveOptimizedPromptForRuntime(
      runtime,
      "media_description",
      imageDescriptionTemplate,
    );
    const described = await describeImageCached(
      runtime,
      `data:${mime};base64,${data}`,
      prompt,
    );
    if (!described) {
      return `[${label} ${mime}, ${size} bytes; no description came back]`;
    }
    return described.text || `${described.title || "Image"}: ${described.description}`;
  } catch (error) {
    return unread(label, mime, size, error);
  }
}

/**
 * Audio, transcribed through the `TRANSCRIPTION` tier from its bytes,
 * as core transcribes an inbound attachment — or named by type and
 * size without that tier.
 */
async function transcribe(runtime, mime, data, label) {
  const bytes = Buffer.from(data, "base64");
  if (typeof runtime.getModel(ModelType.TRANSCRIPTION) !== "function") {
    return `[${label} ${mime}, ${bytes.length} bytes; no transcription model is registered]`;
  }
  try {
    const transcript = await runtime.useModel(ModelType.TRANSCRIPTION, bytes);
    return `Transcript: ${transcript}`;
  } catch (error) {
    return unread(label, mime, bytes.length, error);
  }
}

/** A block the transcoder could not read: said in place. */
function unread(label, mime, size, error) {
  return `[${label} ${mime}, ${size} bytes; could not be read: ${describe(error)}]`;
}

/**
 * The resources, listed into every turn's state — always on, and
 * empty when there are none, so it costs nothing then. `current`
 * reads the cache the notifications keep fresh.
 */
function resourcesProvider(current) {
  return {
    name: RESOURCES_PROVIDER,
    description: `The caller's resources, readable with ${READ_RESOURCE}.`,
    alwaysInResponseState: true,
    get: async () => {
      const resources = current();
      if (resources.length === 0) {
        return { text: "", values: {}, data: { resources: [] } };
      }
      const lines = resources.map((resource) => {
        const name = resource.title ?? resource.name ?? resource.uri;
        const description = resource.description ? `: ${resource.description}` : "";
        const type = resource.mimeType ? ` (${resource.mimeType})` : "";
        return `- ${resource.uri} — ${name}${description}${type}`;
      });
      return {
        text: `Resources you can read with ${READ_RESOURCE}, by uri:\n${lines.join("\n")}`,
        values: {},
        data: { resources },
      };
    },
  };
}

/**
 * `DIVERGE_` and the tool name upper-cased with every character outside
 * `[A-Z0-9]` replaced by `_` — the runtime's name pattern. A collision
 * (two tools folding to one name, or the reserved read) gets a numeric
 * suffix. The name is taken on return.
 */
function actionName(toolName, taken) {
  const base = `DIVERGE_${toolName.toUpperCase().replace(/[^A-Z0-9]/g, "_")}`;
  let name = base;
  for (let n = 2; taken.has(name); n += 1) {
    name = `${base}_${n}`;
  }
  if (!ACTION_NAME.test(name)) {
    throw new Error(`the tool ${toolName} folds to an invalid action name ${name}`);
  }
  taken.add(name);
  return name;
}

/**
 * The tool's input schema as the runtime's `ActionParameter[]`: one per
 * property, each carrying its own JSON Schema, required as the schema
 * says. The runtime turns these back into one tool schema for native
 * tool calling, so the model sees what the MCP server declared.
 */
function parameters(inputSchema) {
  const properties = inputSchema?.properties ?? {};
  const required = new Set(inputSchema?.required ?? []);
  return Object.entries(properties).map(([name, schema]) => ({
    name,
    description: schema?.description ?? name,
    required: required.has(name),
    schema: schema ?? { type: "string" },
  }));
}

/** An error's message, or the error itself as text. */
function describe(error) {
  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }
  return String(error);
}
