/**
 * STUB — the global completion store is disconnected.
 *
 * This store was fed by the global daemon-listener singleton
 * (`/listen` broadcast handlers on `agents/spawn` +
 * `functions/execute/*`), which has been REMOVED: the viewer now uses
 * per-component `/agents/instances/…` listeners instead (see
 * `hooks/useAgentsInstancesList` and `hooks/useAgentInstance`). The
 * conversation popup (`hooks/useAgent` → `ConversationModal`) still
 * imports this surface for its live segment, so the exports remain —
 * permanently empty — until the popup is rebuilt on the per-agent
 * conversation stream. TODO: rebuild ConversationModal on
 * `useAgentInstance`'s conversation blocks and delete this module.
 */
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "@objectiveai/sdk";

/** Every agent-completion chunk shape the store could hold. */
export type AgentCompletionChunk =
  | AgentCompletionsResponseStreamingAgentCompletionChunk
  | VectorCompletionsResponseStreamingAgentCompletionChunk
  | FunctionsExecutionsResponseStreamingReasoningSummaryChunk;

/** The raw, UNMERGED chunk list of one agent's most recent
 * completion segment, in arrival order. */
export type AgentCompletion = readonly AgentCompletionChunk[];

/** One agent's live segment. */
export interface AgentCompletionEntry {
  /** Raw, unmerged chunks in arrival order. */
  chunks: AgentCompletionChunk[];
  /** Bumped each time the segment resets. */
  generation: number;
  /** The ROOT function-execution chunk's response `id`, when fed from
   * `functions/execute/*`; `null` for `agents/spawn` segments. */
  functionExecutionResponseId: string | null;
}

const completions = new Map<string, AgentCompletionEntry>();

/** The current segment for one agent instance hierarchy — always
 * `undefined` (the store is disconnected; see the module docs). */
export function agentCompletion(
  hierarchy: string,
): AgentCompletionEntry | undefined {
  return completions.get(hierarchy);
}

/** The whole store — always empty (disconnected). */
export function agentCompletions(): ReadonlyMap<string, AgentCompletionEntry> {
  return completions;
}

/** Subscribe to one hierarchy's segment changes — never fires
 * (disconnected). Returns a no-op unsubscribe. */
export function subscribeAgentCompletions(
  _hierarchy: string,
  _callback: () => void,
): () => void {
  return () => {};
}
