import { useEffect, useMemo, useState } from "react";
import {
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  agentsLogsListExecute,
  vectorCompletionsResponseStreamingAgentCompletionChunkMerged,
  type CliCommandAgentsLogsListResponseItem,
  type CliCommandAgentsLogsListTarget,
} from "@objectiveai/sdk";
import { websocketExecutor } from "../lib/websocket-executor";
import {
  agentCompletion,
  subscribeAgentCompletions,
  type AgentCompletionChunk,
} from "../listener-handlers/agentCompletions";

/** One agent's full picture: historical logs + the live segment. */
export interface UseAgentResult {
  /** Historical logs (ascending), cut BEFORE the first request row
   * the live segment covers — requests are written prior to any log
   * of their response, so everything from that row on is already
   * represented by `completions`. `null` while the one-off read is
   * in flight. */
  logs: readonly CliCommandAgentsLogsListResponseItem[] | null;
  /** The live segment merged into per-response aggregates: one chunk
   * per response id, in first-seen order. Chunks with the same
   * response id merge into their aggregate; a new response id starts
   * the next aggregate. */
  completions: readonly AgentCompletionChunk[];
}

/** The log row types written when a request lands — the cut markers
 * for live-covered logs. */
const REQUEST_TYPES: ReadonlySet<string> = new Set([
  "agent_completion_request",
  "vector_completion_request",
  "function_execution_request",
]);

const EMPTY_IDS: ReadonlySet<string> = new Set();

/**
 * Follow one agent instance hierarchy: a one-off `agents logs read
 * all` (`agents/logs/list`, `pending: false`) for the historical
 * logs, plus a live subscription on the [`agentCompletions`] store
 * that folds the segment's raw chunks into per-response merged
 * aggregates (SDK merge functions; each raw chunk is consumed exactly
 * once, and a segment reset — a new stream — restarts the fold).
 * Re-renders on every store change for the hierarchy.
 */
export function useAgent(hierarchy: string): UseAgentResult {
  const [rawLogs, setRawLogs] = useState<
    readonly CliCommandAgentsLogsListResponseItem[] | null
  >(null);
  const [completions, setCompletions] = useState<
    readonly AgentCompletionChunk[]
  >([]);
  const [coveringIds, setCoveringIds] =
    useState<ReadonlySet<string>>(EMPTY_IDS);

  useEffect(() => {
    // A new hierarchy starts from scratch.
    setRawLogs(null);
    setCompletions([]);
    setCoveringIds(EMPTY_IDS);
    let cancelled = false;

    // The one-off historical read.
    void (async () => {
      try {
        const executor = await websocketExecutor();
        const items: CliCommandAgentsLogsListResponseItem[] = [];
        for await (const item of agentsLogsListExecute(executor, {
          pending: false,
          targets: [logsTarget(hierarchy)],
        })) {
          // In-band error lines are not log rows.
          if (item.type === "error") continue;
          items.push(item);
        }
        if (!cancelled) {
          setRawLogs(items);
        }
      } catch {
        // Daemon unreachable — no history to show.
        if (!cancelled) {
          setRawLogs([]);
        }
      }
    })();

    // The live fold. Effect-local cursor state makes the dedup
    // guarantee: a raw chunk is merged exactly once, and only a
    // generation change (a new stream's segment) restarts the fold.
    let generation = -1;
    let cursor = 0;
    let aggregates: AgentCompletionChunk[] = [];
    const consume = () => {
      const entry = agentCompletion(hierarchy);
      if (entry === undefined) return;
      if (entry.generation !== generation) {
        generation = entry.generation;
        cursor = 0;
        aggregates = [];
      }
      if (cursor >= entry.chunks.length) return;
      for (; cursor < entry.chunks.length; cursor++) {
        const chunk = entry.chunks[cursor];
        const last =
          aggregates.length > 0
            ? aggregates[aggregates.length - 1]
            : undefined;
        if (last !== undefined && last.id === chunk.id) {
          aggregates[aggregates.length - 1] = mergeChunk(last, chunk);
        } else {
          aggregates.push(chunk);
        }
      }
      setCompletions([...aggregates]);
      const ids = new Set(aggregates.map((aggregate) => aggregate.id));
      if (entry.functionExecutionResponseId !== null) {
        ids.add(entry.functionExecutionResponseId);
      }
      setCoveringIds(ids);
    };
    // Catch a segment that existed before this caller mounted.
    consume();
    const unsubscribe = subscribeAgentCompletions(hierarchy, consume);
    return () => {
      cancelled = true;
      unsubscribe();
    };
  }, [hierarchy]);

  // Cut the history where live coverage begins: the FIRST request row
  // whose response id the live segment covers (its own completion ids,
  // or the whole function execution's id) starts the live-covered
  // suffix. Reactive — a stream arriving after the read re-cuts.
  const logs = useMemo(() => {
    if (rawLogs === null) return null;
    const cut = rawLogs.findIndex(
      (item) =>
        REQUEST_TYPES.has(item.type) && coveringIds.has(item.response_id),
    );
    return cut === -1 ? rawLogs : rawLogs.slice(0, cut);
  }, [rawLogs, coveringIds]);

  return { logs, completions };
}

/** Merge one same-response-id chunk into its aggregate. The vector
 * shape (the only member carrying `index`) merges via the vector
 * function; the agent and reasoning shapes are structurally the
 * agent chunk and merge via the agent function. */
function mergeChunk(
  a: AgentCompletionChunk,
  b: AgentCompletionChunk,
): AgentCompletionChunk {
  if ("index" in a && "index" in b) {
    return vectorCompletionsResponseStreamingAgentCompletionChunkMerged(
      a,
      b,
    )[0];
  }
  return agentCompletionsResponseStreamingAgentCompletionChunkMerged(a, b)[0];
}

/** The logs-list target for one AIH: leaf after the last `/`, lineage
 * prefix as the parent (`null` for a bare single-segment hierarchy). */
function logsTarget(hierarchy: string): CliCommandAgentsLogsListTarget {
  const slash = hierarchy.lastIndexOf("/");
  if (slash === -1) {
    return {
      by: "direct",
      agent_instance: hierarchy,
      parent_agent_instance_hierarchy: null,
    };
  }
  return {
    by: "direct",
    agent_instance: hierarchy.slice(slash + 1),
    parent_agent_instance_hierarchy: hierarchy.slice(0, slash),
  };
}
