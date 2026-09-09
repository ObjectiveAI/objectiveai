/**
 * The harness's driver for one Eliza run: one process, one
 * `AgentRuntime`, driven over JSON lines on stdin and answering on
 * stdout, one object per line. The harness (the Rust program beside
 * this project) writes `configure` once, then `turn`, `read` and `stop`;
 * this entry answers `ready`, the turn's stream (`text`, `tool_call`,
 * `tool_result`, `usage`, `notification`), `done`, `value` and
 * `stopped`. Anything that fails before `ready` is `fatal` and the
 * process exits 1; after it, a failure is a `notification` or the
 * `failure` of a `done`. stderr is Eliza's logger and is never parsed.
 *
 * Settings come in the `configure` line's map and are handed to the
 * runtime constructor (the core's `getSetting` never reads the process
 * environment); the same values are already in this process's
 * environment for the plugins that look there. `runtime.stop()` is
 * called in its ordinary mode, never fast: the post-turn work is the
 * lineage's memory, and it lands in the database only when the runtime
 * is allowed to finish it.
 */

import { randomUUID } from "node:crypto";
import { createInterface } from "node:readline";

import {
  AgentRuntime,
  ChannelType,
  EventType,
  ModelType,
  createMessageMemory,
  createUniqueUuid,
} from "@elizaos/core";

import { createDivergePlugin } from "./diverge.mjs";

/** The external ids every Eliza id of the lineage derives from. */
const ROOM = "diverge";
const USER = "diverge:user";

/** The structured envelopes `onStreamChunk` may carry as JSON text. */
const STREAM_EVENT_TYPES = new Set([
  "tool_call",
  "tool_result",
  "evaluation",
  "context_event",
]);

/**
 * Actions whose envelopes are not tool calls to report: the reply is
 * the text itself, and the diverge plugin reports its own calls.
 */
const SILENT_ACTIONS = new Set(["REPLY", "IGNORE", "NONE"]);
const DIVERGE_PREFIX = "DIVERGE_";

/** One protocol line out. */
function emit(line) {
  process.stdout.write(`${JSON.stringify(line)}\n`);
}

/** An error's message, or the error itself as text. */
function describe(error) {
  if (error && typeof error === "object" && "message" in error) {
    return String(error.message);
  }
  return String(error);
}

/** Before `ready` there is nothing to salvage: say why, and leave. */
function fatal(error) {
  emit({ type: "fatal", error: describe(error) });
  process.exit(1);
}

/**
 * Import one package from this project and find the plugin it exports,
 * the way the elizaOS host does: `default`, then `plugin`, then any
 * named export that looks like a plugin, `*Plugin` names first.
 */
async function loadPlugin(name) {
  const mod = await import(name);
  if (looksLikePlugin(mod.default)) return mod.default;
  if (looksLikePlugin(mod.plugin)) return mod.plugin;
  const keys = Object.keys(mod).filter(
    (key) => key !== "default" && key !== "plugin",
  );
  const preferred = keys.filter(
    (key) => /plugin$/i.test(key) || /^plugin/i.test(key),
  );
  const rest = keys.filter((key) => !preferred.includes(key));
  for (const key of [...preferred, ...rest]) {
    if (looksLikePlugin(mod[key])) return mod[key];
  }
  throw new Error(`${name} exports no elizaOS plugin`);
}

/** The host's shape test: a name, a description, and one capability. */
function looksLikePlugin(value) {
  if (!value || typeof value !== "object") return false;
  if (typeof value.name !== "string" || typeof value.description !== "string") {
    return false;
  }
  return (
    Array.isArray(value.services) ||
    Array.isArray(value.providers) ||
    Array.isArray(value.actions) ||
    Array.isArray(value.routes) ||
    Array.isArray(value.events) ||
    Array.isArray(value.views) ||
    typeof value.init === "function"
  );
}

/**
 * `plugin-openai`, shaped to the agent: its embedding tier is never
 * registered — the agent's `embedding` structure is the one source of
 * vectors. Its media tiers (image description and generation,
 * transcription, speech), which its `init` registers against the
 * caller's endpoint, always are: they are how a tool's image or audio
 * is read to the model. Whether the model may GENERATE media is the
 * `GENERATE_MEDIA` action's, gated after initialize.
 */
function shapeOpenai(plugin) {
  const models = { ...(plugin.models ?? {}) };
  delete models[ModelType.TEXT_EMBEDDING];
  return { ...plugin, models };
}

/** The runtime, and everything a turn needs of it. */
let runtime = null;
let entityId = null;
let roomId = null;
/** Whether any `usage` line went out during the current turn. */
let usageSeen = false;
/** Eliza's own vault, opened on the first read of it. */
let elizaVault = null;

async function configure(config) {
  const plugins = [];
  for (const name of config.plugins) {
    const plugin = await loadPlugin(name);
    plugins.push(
      name === "@elizaos/plugin-openai" ? shapeOpenai(plugin) : plugin,
    );
  }
  const diverge = await createDivergePlugin({ url: config.mcpUrl, emit });
  plugins.push(diverge.plugin);
  for (const name of config.installed) {
    plugins.push(await loadPlugin(name));
  }

  runtime = new AgentRuntime({
    agentId: config.agentId,
    character: config.character,
    plugins,
    settings: config.settings,
    advancedCapabilities: config.advancedCapabilities,
    enableRelationships: config.enableRelationships,
    enableDocuments: config.enableDocuments,
    checkShouldRespond: false,
    logLevel: "warn",
  });

  // The bill: one usage line per model call, as the provider plugin
  // reports it. The chunk it becomes is a delta, so no summing here.
  runtime.registerEvent(EventType.MODEL_USED, async (payload) => {
    const tokens = payload?.tokens;
    if (!tokens) {
      emit({
        type: "notification",
        message: {
          kind: "usage",
          error: "a model call reported no token counts",
          model: payload?.model ?? null,
        },
      });
      return;
    }
    usageSeen = true;
    const prompt = tokens.prompt ?? 0;
    const completion = tokens.completion ?? 0;
    emit({
      type: "usage",
      prompt,
      completion,
      total: tokens.total ?? prompt + completion,
    });
  });

  await runtime.initialize();
  if (!runtime.messageService) {
    throw new Error("the runtime has no message service after initialize");
  }
  // A list-changed notification that arrived before the runtime was
  // ready is applied now, with the static actions certainly registered.
  await diverge.flush();
  // The generate_media switch is the action: off, the model cannot
  // ask for media, while the tiers that READ media stay registered.
  if (!config.generateMedia) {
    runtime.unregisterAction("GENERATE_MEDIA");
  }

  entityId = createUniqueUuid(runtime, USER);
  roomId = createUniqueUuid(runtime, ROOM);
  await runtime.ensureConnection({
    entityId,
    roomId,
    userName: "user",
    source: "diverge",
    channelId: ROOM,
    type: ChannelType.DM,
  });
}

/** The envelope a chunk string is, if it is one of the four. */
function structuredEvent(chunk) {
  const trimmed = chunk.trimStart();
  if (!trimmed.startsWith("{")) return null;
  try {
    const parsed = JSON.parse(trimmed);
    if (
      parsed &&
      typeof parsed.type === "string" &&
      STREAM_EVENT_TYPES.has(parsed.type)
    ) {
      return parsed;
    }
  } catch {
    // Not JSON: plain text that happens to start with a brace.
  }
  return null;
}

/** An envelope's tool name, from the keys the adapters use. */
function toolName(event) {
  const call = event.toolCall ?? {};
  return call.name ?? call.toolName ?? call.tool ?? call.action ?? "";
}

/** Whether an envelope's action is one this entry reports. */
function reported(name) {
  return !SILENT_ACTIONS.has(name) && !name.startsWith(DIVERGE_PREFIX);
}

/** The runtime's tool envelopes and evaluations, as protocol lines. */
function report(event) {
  switch (event.type) {
    case "tool_call": {
      const name = toolName(event);
      if (!reported(name)) return;
      const call = event.toolCall ?? {};
      const args = call.arguments ?? call.args ?? call.input ?? call.params ?? {};
      emit({
        type: "tool_call",
        id: call.id ?? call.toolCallId ?? randomUUID(),
        name,
        arguments: typeof args === "string" ? args : JSON.stringify(args),
      });
      return;
    }
    case "tool_result": {
      const name = toolName(event);
      if (!reported(name)) return;
      const call = event.toolCall ?? {};
      const result = event.result ?? call.result;
      emit({
        type: "tool_result",
        id: event.toolCallId ?? call.id ?? call.toolCallId ?? randomUUID(),
        result: {
          content: [
            {
              type: "text",
              text:
                typeof result === "string"
                  ? result
                  : JSON.stringify(result ?? null),
            },
          ],
          isError: (event.status ?? call.status) === "failed",
        },
      });
      return;
    }
    case "evaluation":
      emit({
        type: "notification",
        message: { kind: "evaluation", evaluation: event.evaluation ?? null },
      });
      return;
    case "context_event":
      emit({
        type: "notification",
        message: { kind: "context_event", event: event.event ?? null },
      });
      return;
    default:
      return;
  }
}

async function turn(text) {
  usageSeen = false;
  let streamed = "";
  let spoke = false;

  const onStreamChunk = (chunk, _messageId, accumulated) => {
    // A structured field stream: `accumulated` is the truth and may
    // restart; the delta is what it grew by, else this chunk.
    if (typeof accumulated === "string") {
      if (accumulated === streamed) return;
      const delta = accumulated.startsWith(streamed)
        ? accumulated.slice(streamed.length)
        : chunk;
      streamed = accumulated;
      if (delta) {
        spoke = true;
        emit({ type: "text", delta });
      }
      return;
    }
    const event = structuredEvent(chunk);
    if (event) {
      report(event);
      return;
    }
    if (chunk) {
      streamed += chunk;
      spoke = true;
      emit({ type: "text", delta: chunk });
    }
  };

  const memory = createMessageMemory({
    id: randomUUID(),
    entityId,
    roomId,
    content: { text, source: "diverge", channelType: ChannelType.DM },
  });

  const controller = new AbortController();
  let result = null;
  let thrown = null;
  try {
    result = await runtime.messageService.handleMessage(
      runtime,
      memory,
      async () => [],
      { abortSignal: controller.signal, onStreamChunk },
    );
  } catch (error) {
    thrown = error;
  }

  if (!usageSeen) {
    emit({
      type: "notification",
      message: { kind: "usage", error: "no model usage was reported this turn" },
    });
  }

  let failure = null;
  if (thrown) {
    failure = { kind: "thrown", transient: false, message: describe(thrown) };
  } else if (result?.terminalFailure) {
    failure = result.terminalFailure;
  }
  const content = result?.responseContent;
  const finalText =
    content &&
    content.transcriptVisibility !== "internal" &&
    typeof content.text === "string"
      ? content.text
      : null;
  emit({ type: "done", text: finalText, streamed: spoke, failure });
}

/** A rotating secret read back from where the plugin left it. */
async function read(kind, key) {
  let value = null;
  if (kind === "setting") {
    const setting = runtime.getSetting(key);
    if (setting !== null && setting !== undefined) {
      value = typeof setting === "string" ? setting : JSON.stringify(setting);
    }
  } else if (kind === "vault") {
    if (!elizaVault) {
      const { createVault } = await import("@elizaos/vault");
      elizaVault = createVault();
    }
    try {
      value = await elizaVault.get(key);
    } catch (error) {
      // Absent is the honest answer, and the harness treats it as
      // nothing new. Any other failure is said, and answers nothing.
      if (error?.name !== "VaultMissError") {
        emit({
          type: "notification",
          message: { kind: "vault", key, error: describe(error) },
        });
      }
    }
  }
  emit({ type: "value", key, value });
}

async function stop() {
  if (runtime) {
    await runtime.stop();
    await runtime.close();
  }
  emit({ type: "stopped" });
  process.exit(0);
}

async function main() {
  const lines = createInterface({
    input: process.stdin,
    crlfDelay: Number.POSITIVE_INFINITY,
  });
  let configured = false;
  for await (const line of lines) {
    if (!line.trim()) continue;
    let request;
    try {
      request = JSON.parse(line);
    } catch (error) {
      if (!configured) fatal(error);
      emit({
        type: "notification",
        message: { kind: "protocol", error: describe(error) },
      });
      continue;
    }
    if (!configured) {
      if (request.type !== "configure") {
        fatal(new Error(`expected configure, got ${request.type}`));
      }
      try {
        await configure(request);
      } catch (error) {
        fatal(error);
      }
      configured = true;
      emit({ type: "ready" });
      continue;
    }
    switch (request.type) {
      case "turn":
        await turn(request.text);
        break;
      case "read":
        await read(request.kind, request.key);
        break;
      case "stop":
        await stop();
        return;
      default:
        emit({
          type: "notification",
          message: { kind: "protocol", error: `unknown request ${request.type}` },
        });
    }
  }
  // stdin closed without a stop: the harness is gone. Stop anyway, so
  // the database is left consistent.
  await stop();
}

main().catch((error) => {
  emit({
    type: "notification",
    message: { kind: "entry", error: describe(error) },
  });
  process.exit(1);
});
