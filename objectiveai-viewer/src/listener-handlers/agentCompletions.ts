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

/** One agent's live segment: the raw chunks of the most recent
 * streaming execution that touched it — one stream = one segment. */
export interface AgentCompletionEntry {
  /** Raw, unmerged chunks in arrival order (mutated in place as
   * chunks arrive). May span multiple response ids: within one
   * stream, a new response id does NOT reset the segment. */
  chunks: AgentCompletionChunk[];
  /** Globally monotonic; bumped each time the segment resets (a new
   * execution's first chunk for this AIH). Consumers folding the
   * chunks incrementally compare it to detect resets and restart
   * their cursors. */
  generation: number;
  /** The ROOT function-execution chunk's response `id` when the
   * segment is fed from `functions/execute/*`; `null` for
   * `agents/spawn` segments. The log row for that request is written
   * before every log row of the whole execution, so this id marks
   * where historical logs end and live-covered logs begin. */
  functionExecutionResponseId: string | null;
}

// ── The GLOBAL completion store ─────────────────────────────────────
// One store for the whole app: a single pair of execution-handler
// registrations on the daemon-listener singleton (register at viewer
// startup, before the listener starts) keeps, per agent instance
// hierarchy, the raw chunk list of that agent's MOST RECENT
// completion segment. Chunks come from streaming `agents/spawn`
// executions directly, and from streaming `functions/execute/*`
// executions by recursively walking each function-execution chunk for
// the agent completions nested inside (vector completion tasks'
// `completions`, reasoning summaries, nested function tasks — at any
// depth). No merging — consumers fold the chunks however they like.
// One stream = one segment: an execution's FIRST chunk for an AIH
// resets that entry, everything else appends. Entries persist after
// streams end; per-AIH subscribers are notified on every change.

/** AIH → the current segment. */
const completions = new Map<string, AgentCompletionEntry>();
/** AIH → change callbacks, fired after each recorded chunk. */
const subscribers = new Map<string, Set<() => void>>();
let nextGeneration = 0;
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
    // Hierarchies this execution has claimed: its FIRST chunk for an
    // AIH resets that agent's segment, subsequent chunks append.
    const claimed = new Set<string>();
    return (item) => {
      // Not chunks: the bare-string `Id` (AIH announcement) item and
      // in-band error items — only the completion chunk carries its
      // slot's hierarchy.
      if (typeof item === "string") return;
      if ("agent_instance_hierarchy" in item) {
        record(item, claimed, null);
      }
    };
  });
  registerExecutionHandler(
    ["functions/execute/standard", "functions/execute/swiss_system"] as const,
    (execution) => {
      if (execution.request.dangerous_advanced?.stream !== true) return;
      const claimed = new Set<string>();
      return (item) => {
        // Not chunks: the bare-string execution-id item, the tagged
        // AIH announcement, and in-band error items — only the
        // function-execution chunk carries an `object` tag.
        if (typeof item === "string") return;
        if ("object" in item) {
          collectAgentCompletions(item, claimed, item.id);
        }
      };
    },
  );
}

/** The current segment for one agent instance hierarchy, or
 * `undefined` if nothing has chunked it yet. */
export function agentCompletion(
  hierarchy: string,
): AgentCompletionEntry | undefined {
  return completions.get(hierarchy);
}

/** The whole store: AIH → current segment. */
export function agentCompletions(): ReadonlyMap<string, AgentCompletionEntry> {
  return completions;
}

/**
 * Subscribe to one hierarchy's segment changes (fired after each
 * recorded chunk, including the segment-resetting first chunk of a
 * new stream). Returns the unsubscribe function.
 */
export function subscribeAgentCompletions(
  hierarchy: string,
  callback: () => void,
): () => void {
  let set = subscribers.get(hierarchy);
  if (!set) {
    set = new Set();
    subscribers.set(hierarchy, set);
  }
  set.add(callback);
  return () => {
    const current = subscribers.get(hierarchy);
    if (current) {
      current.delete(callback);
      if (current.size === 0) {
        subscribers.delete(hierarchy);
      }
    }
  };
}

/** Store one chunk under its hierarchy. One stream = one segment: the
 * execution's first chunk for an AIH (tracked by `claimed`) replaces
 * the entry with a fresh segment; every subsequent chunk appends,
 * response-id changes included. Subscribers fire per chunk. */
function record(
  chunk: AgentCompletionChunk,
  claimed: Set<string>,
  functionExecutionResponseId: string | null,
): void {
  const hier = chunk.agent_instance_hierarchy;
  let entry = completions.get(hier);
  if (!claimed.has(hier) || entry === undefined) {
    claimed.add(hier);
    entry = {
      chunks: [],
      generation: nextGeneration++,
      functionExecutionResponseId,
    };
    completions.set(hier, entry);
  }
  entry.chunks.push(chunk);
  const set = subscribers.get(hier);
  if (set) {
    for (const callback of [...set]) {
      callback();
    }
  }
}

/** Recursively walk one function-execution chunk (root or nested
 * function task — same shape either way), recording every agent
 * completion inside: the reasoning summary, each vector completion
 * task's per-agent `completions`, and nested function tasks all the
 * way down. The task union discriminates on `object` —
 * `"vector.completion.chunk"` versus the function-execution values.
 * `functionExecutionResponseId` is the ROOT chunk's id, threaded
 * unchanged through the recursion. */
function collectAgentCompletions(
  chunk:
    | FunctionsExecutionsResponseStreamingFunctionExecutionChunk
    | FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  claimed: Set<string>,
  functionExecutionResponseId: string,
): void {
  if (chunk.reasoning != null) {
    record(chunk.reasoning, claimed, functionExecutionResponseId);
  }
  for (const task of chunk.tasks) {
    if (task.object === "vector.completion.chunk") {
      for (const completion of task.completions) {
        record(completion, claimed, functionExecutionResponseId);
      }
    } else {
      collectAgentCompletions(task, claimed, functionExecutionResponseId);
    }
  }
}
