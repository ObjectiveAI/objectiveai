/**
 * The `diverge` plugin: the caller's MCP tools, each registered as one
 * native Eliza action. Built before the runtime is constructed, inside
 * `POST /run`, from the proxy's MCP server at the URL the harness hands
 * over — the one place this project dials the proxy. Every call goes
 * through the MCP client here, and the call and its result are reported
 * on the harness's line protocol by this plugin itself, byte-faithfully:
 * the runtime's own tool envelopes for these actions are dropped by the
 * entry, so each call is said exactly once, from the side that has the
 * bytes. `ActionResult.text` is the tool's text blocks joined, which is
 * what the model reads back; no extra model call paraphrases it.
 */

import { randomUUID } from "node:crypto";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

/** The action-name pattern the runtime enforces. */
const ACTION_NAME = /^[A-Z_][A-Z0-9_]*$/;

/**
 * Connect to the proxy's MCP server, list every tool it serves, and
 * return the plugin. `emit` takes one protocol line (an object) and
 * writes it to the harness.
 */
export async function createDivergePlugin({ url, emit }) {
  const client = new Client({
    name: "diverge-agentic-loop-eliza",
    version: "2.3.0",
  });
  await client.connect(new StreamableHTTPClientTransport(new URL(url)));

  const tools = [];
  let cursor;
  do {
    const page = await client.listTools(cursor ? { cursor } : undefined);
    tools.push(...(page.tools ?? []));
    cursor = page.nextCursor;
  } while (cursor);

  const taken = new Set();
  const actions = tools.map((tool) => action(client, emit, tool, taken));
  return {
    name: "diverge",
    description:
      "The caller's MCP tools, through the Diverge container proxy",
    actions,
  };
}

/** One tool as one action. */
function action(client, emit, tool, taken) {
  const name = actionName(tool.name, taken);
  return {
    name,
    description: tool.description ?? tool.name,
    parameters: parameters(tool.inputSchema),
    validate: async () => true,
    handler: async (_runtime, _message, _state, options) => {
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
      const text = (result.content ?? [])
        .filter((block) => block.type === "text")
        .map((block) => block.text)
        .join("\n");
      return {
        success: !result.isError,
        text,
        data: { mcp: result },
      };
    },
  };
}

/**
 * `DIVERGE_` and the tool name upper-cased with every character outside
 * `[A-Z0-9]` replaced by `_` — the runtime's name pattern. A collision
 * (two tools folding to one name) gets a numeric suffix.
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
