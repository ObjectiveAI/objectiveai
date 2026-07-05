import type { AgentCompletionsResponseStreamingAgentCompletionChunk } from "@objectiveai/sdk";
import { registerExecutionHandler } from "../daemon-listener";

/** The raw, UNMERGED chunk list of one agent's most recent
 * conversation segment, in arrival order. */
export type AgentConversation =
  readonly AgentCompletionsResponseStreamingAgentCompletionChunk[];

// ── The GLOBAL conversation store ───────────────────────────────────
// One store for the whole app: a single execution-handler registration
// on the daemon-listener singleton (register at viewer startup, before
// the listener starts) keeps, per agent instance hierarchy, the raw
// chunk list of that agent's MOST RECENT streaming `agents/spawn`
// execution. No merging — consumers fold the chunks however they
// like. Entries persist after the stream ends (the latest segment
// stays readable); a newer execution chunking the same hierarchy
// REPLACES the old segment wholesale.

/** AIH → the most recent segment's chunk list (mutated in place as
 * chunks arrive; replaced when a newer execution claims the AIH). */
const conversations = new Map<
  string,
  AgentCompletionsResponseStreamingAgentCompletionChunk[]
>();
let registered = false;

/**
 * Register the conversation store's execution handler on the
 * daemon-listener singleton (idempotent, app-lifetime). Call at
 * viewer startup, BEFORE `startDaemonListener()` — the singleton is
 * live-only, and a late registration misses everything announced
 * before it existed.
 */
export function registerAgentConversationsHandler(): void {
  if (registered) return;
  registered = true;
  registerExecutionHandler("agents/spawn", (execution) => {
    // Streaming form only; the unary variant carries no chunks.
    if (execution.request.dangerous_advanced?.stream !== true) return;
    // Hierarchies this execution has claimed: its FIRST chunk for an
    // AIH replaces that agent's stored segment, subsequent chunks
    // append to it.
    const claimed = new Set<string>();
    return (item) => {
      const chunk = asChunk(item);
      if (chunk === null) return;
      const hier = chunk.agent_instance_hierarchy;
      if (!claimed.has(hier)) {
        claimed.add(hier);
        conversations.set(hier, []);
      }
      conversations.get(hier)?.push(chunk);
    };
  });
}

/** The most recent conversation segment for one agent instance
 * hierarchy, or `undefined` if no streaming spawn has chunked it. */
export function agentConversation(
  hierarchy: string,
): AgentConversation | undefined {
  return conversations.get(hierarchy);
}

/** The whole store: AIH → most recent segment. */
export function agentConversations(): ReadonlyMap<string, AgentConversation> {
  return conversations;
}

/** Narrow one spawn response item to a completion chunk. Bare-string
 * `Id` (AIH announcement) items and in-band `{type: "error"}` items
 * are not chunks; a chunk always carries its slot's
 * `agent_instance_hierarchy`. */
function asChunk(
  item: unknown,
): AgentCompletionsResponseStreamingAgentCompletionChunk | null {
  if (
    item !== null &&
    typeof item === "object" &&
    typeof (item as { agent_instance_hierarchy?: unknown })
      .agent_instance_hierarchy === "string"
  ) {
    return item as AgentCompletionsResponseStreamingAgentCompletionChunk;
  }
  return null;
}
