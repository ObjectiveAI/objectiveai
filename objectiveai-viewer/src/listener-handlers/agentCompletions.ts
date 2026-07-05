import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "@objectiveai/sdk";
import { registerExecutionHandler } from "../daemon-listener";

/** Every agent-completion chunk shape the store can hold: `agents
 * spawn`'s own chunks, the per-agent chunks nested inside a function
 * execution's vector completion tasks, and reasoning summary chunks
 * (an agent completion in their own right). All carry the slot's
 * `agent_instance_hierarchy` and the completion's response `id`. */
export type AgentCompletionChunk =
  | AgentCompletionsResponseStreamingAgentCompletionChunk
  | VectorCompletionsResponseStreamingAgentCompletionChunk
  | FunctionsExecutionsResponseStreamingReasoningSummaryChunk;

/** The raw, UNMERGED chunk list of one agent's most recent
 * completion segment, in arrival order. */
export type AgentCompletion = readonly AgentCompletionChunk[];

// ── The GLOBAL completion store ─────────────────────────────────────
// One store for the whole app: a single pair of execution-handler
// registrations on the daemon-listener singleton (register at viewer
// startup, before the listener starts) keeps, per agent instance
// hierarchy, the raw chunk list of that agent's MOST RECENT
// completion. Chunks come from streaming `agents/spawn` executions
// directly, and from streaming `functions/execute/*` executions by
// recursively walking each function-execution chunk for the agent
// completions nested inside (vector completion tasks' `completions`,
// reasoning summaries, nested function tasks — at any depth). No
// merging — consumers fold the chunks however they like. Entries
// persist after streams end; the response `id` is the whole reset
// story (see `record`).

/** AIH → the most recent segment's chunk list (mutated in place while
 * the response id holds; replaced from scratch when it changes). */
const completions = new Map<string, AgentCompletionChunk[]>();
let registered = false;

/**
 * Register the completion store's execution handlers on the
 * daemon-listener singleton (idempotent, app-lifetime). Call at
 * viewer startup, BEFORE `startDaemonListener()` — the singleton is
 * live-only, and a late registration misses everything announced
 * before it existed.
 */
export function registerAgentCompletionsHandler(): void {
  if (registered) return;
  registered = true;
  registerExecutionHandler("agents/spawn", (execution) => {
    // Streaming form only; the unary variant carries no chunks.
    if (execution.request.dangerous_advanced?.stream !== true) return;
    return (item) => {
      // Not chunks: the bare-string `Id` (AIH announcement) item and
      // in-band error items — only the completion chunk carries its
      // slot's hierarchy.
      if (typeof item === "string") return;
      if ("agent_instance_hierarchy" in item) {
        record(item);
      }
    };
  });
  registerExecutionHandler(
    ["functions/execute/standard", "functions/execute/swiss_system"] as const,
    (execution) => {
      if (execution.request.dangerous_advanced?.stream !== true) return;
      return (item) => {
        // Not chunks: the bare-string execution-id item, the tagged
        // AIH announcement, and in-band error items — only the
        // function-execution chunk carries an `object` tag.
        if (typeof item === "string") return;
        if ("object" in item) {
          collectAgentCompletions(item);
        }
      };
    },
  );
}

/** The most recent completion segment for one agent instance
 * hierarchy, or `undefined` if nothing has chunked it yet. */
export function agentCompletion(
  hierarchy: string,
): AgentCompletion | undefined {
  return completions.get(hierarchy);
}

/** The whole store: AIH → most recent segment. */
export function agentCompletions(): ReadonlyMap<string, AgentCompletion> {
  return completions;
}

/** Store one chunk under its hierarchy. The segment accumulates only
 * while the completion's response `id` holds: a chunk whose `id`
 * differs from the segment's last stored chunk resets the entry to
 * scratch with the new solo chunk (a fresh execution — or a fresh
 * multi-turn segment inside one — always supersedes). */
function record(chunk: AgentCompletionChunk): void {
  const hier = chunk.agent_instance_hierarchy;
  const existing = completions.get(hier);
  if (
    existing !== undefined &&
    existing.length > 0 &&
    existing[existing.length - 1].id === chunk.id
  ) {
    existing.push(chunk);
  } else {
    completions.set(hier, [chunk]);
  }
}

/** Recursively walk one function-execution chunk (root or nested
 * function task — same shape either way), recording every agent
 * completion inside: the reasoning summary, each vector completion
 * task's per-agent `completions`, and nested function tasks all the
 * way down. The task union discriminates on `object` —
 * `"vector.completion.chunk"` versus the function-execution values. */
function collectAgentCompletions(
  chunk:
    | FunctionsExecutionsResponseStreamingFunctionExecutionChunk
    | FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
): void {
  if (chunk.reasoning != null) {
    record(chunk.reasoning);
  }
  for (const task of chunk.tasks) {
    if (task.object === "vector.completion.chunk") {
      for (const completion of task.completions) {
        record(completion);
      }
    } else {
      collectAgentCompletions(task);
    }
  }
}
