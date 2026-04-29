import { useState } from "react";
import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "objectiveai";
import { AgentCompletionChat } from "./AgentCompletionView";
import { VoteMatrix } from "./VoteMatrix";
import { scoreColor, pct, stateColor } from "./judgment-utils";

interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: { id: string };
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

// ---------------------------------------------------------------------------
// Tree types
// ---------------------------------------------------------------------------

interface VoteData {
  agent: string;
  vote: number[];
  weight: number;
  from_cache?: boolean | null;
  swarm_index?: number;
  flat_swarm_index?: number;
}

interface TaskLeaf {
  key: string;
  label: string;
  taskPath: number[];
  votes: VoteData[];
  scores: number[];
  weights: number[];
  completions: Array<{
    chunk: VectorCompletionsResponseStreamingAgentCompletionChunk;
    error: { code: number; message: unknown } | null;
  }>;
}

interface ReasoningLeaf {
  key: string;
  label: string;
  chunk: VectorCompletionsResponseStreamingAgentCompletionChunk;
  error: { code: number; message: unknown } | null;
}

type Leaf = { type: "task"; data: TaskLeaf } | { type: "reasoning"; data: ReasoningLeaf };

// ---------------------------------------------------------------------------
// Tree walk
// ---------------------------------------------------------------------------

function isVectorCompletionTask(t: unknown): boolean {
  return (
    typeof t === "object" &&
    t !== null &&
    "object" in t &&
    (t as { object?: unknown }).object === "vector.completion.chunk"
  );
}

function isFunctionExecutionTask(t: unknown): boolean {
  if (typeof t !== "object" || t === null || !("object" in t)) return false;
  const o = (t as { object?: unknown }).object;
  return (
    o === "scalar.function.execution.chunk" ||
    o === "vector.function.execution.chunk"
  );
}

function modifiersFor(
  fe: FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
): string[] {
  const mods: string[] = [];
  if (fe.split_index !== undefined && fe.split_index !== null) {
    mods.push(`split=${fe.split_index}`);
  }
  if (fe.swiss_pool_index !== undefined && fe.swiss_pool_index !== null) {
    mods.push(`pool=${fe.swiss_pool_index}`);
  }
  if (fe.swiss_round !== undefined && fe.swiss_round !== null) {
    mods.push(`round=${fe.swiss_round}`);
  }
  return mods;
}

function formatLabel(path: number[], modifiers: string[]): string {
  const p = path.length === 0 ? "root" : `task ${path.join(".")}`;
  return modifiers.length === 0 ? p : `${p}  [${modifiers.join(", ")}]`;
}

function emitReasoning(
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunk | null | undefined,
  path: number[],
  modifiers: string[],
  out: Leaf[],
): void {
  if (!reasoning) return;
  const r = reasoning as unknown as VectorCompletionsResponseStreamingAgentCompletionChunk & {
    error?: { code: number; message: unknown } | null;
  };
  const compError = reasoning.error
    ? { code: reasoning.error.code, message: reasoning.error.message }
    : null;
  out.push({
    type: "reasoning",
    data: {
      key: `reasoning-${r.id || `${path.join(".")}-${modifiers.join(",")}`}`,
      label: `${formatLabel(path, modifiers)} — reasoning`,
      chunk: r,
      error: compError,
    },
  });
}

function walkTasks(
  tasks: FunctionsExecutionsResponseStreamingTaskChunk[] | undefined,
  inheritedModifiers: string[],
  out: Leaf[],
): void {
  if (!tasks) return;
  for (const t of tasks) {
    if (isVectorCompletionTask(t)) {
      const v = t as unknown as FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk;
      const path = v.task_path ?? [];
      const completions = (v.completions ?? []).map((comp) => ({
        chunk: comp,
        error: comp.error
          ? { code: comp.error.code, message: comp.error.message }
          : null,
      }));
      const votes = (v.votes ?? []) as VoteData[];
      out.push({
        type: "task",
        data: {
          key: `task-${path.join(".")}-${v.id}`,
          label: formatLabel(path, inheritedModifiers),
          taskPath: path,
          votes,
          scores: v.scores ?? [],
          weights: v.weights ?? [],
          completions,
        },
      });
      continue;
    }
    if (isFunctionExecutionTask(t)) {
      const fe = t as unknown as FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk;
      const mods = [...inheritedModifiers, ...modifiersFor(fe)];
      emitReasoning(fe.reasoning, fe.task_path ?? [], mods, out);
      walkTasks(fe.tasks, mods, out);
      continue;
    }
  }
}

function collectLeaves(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): Leaf[] {
  const out: Leaf[] = [];
  emitReasoning(chunk.reasoning, [], [], out);
  walkTasks(chunk.tasks, [], out);
  return out;
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function TaskCard({ leaf }: { leaf: TaskLeaf }) {
  const [showDetail, setShowDetail] = useState(false);
  const hasVotes = leaf.votes.length > 0;
  const hasScores = leaf.scores.length > 0;
  const maxScore = hasScores ? Math.max(...leaf.scores) : 0;
  const winnerIdx = hasScores ? leaf.scores.indexOf(maxScore) : -1;

  return (
    <div className="exec-task">
      <div className="exec-task-header" onClick={() => setShowDetail(!showDetail)}>
        <span className={`exec-arrow${showDetail ? " exec-arrow-open" : ""}`}>
          &#x25B8;
        </span>
        <span className="exec-task-path">{leaf.label}</span>
        {hasScores && (
          <span className="exec-task-scores">
            {leaf.scores.map((s, i) => (
              <span
                key={i}
                className={i === winnerIdx ? "exec-score-winner" : "exec-score"}
              >
                {pct(s)}
              </span>
            ))}
          </span>
        )}
        <span className="exec-task-agents">{leaf.completions.length} agents</span>
      </div>

      {hasVotes && (
        <div className="exec-task-votes">
          <VoteMatrix votes={leaf.votes} scores={leaf.scores} />
        </div>
      )}

      {!hasVotes && hasScores && (
        <div className="exec-task-score-bar">
          {leaf.scores.map((s, i) => (
            <div
              key={i}
              className="exec-score-seg"
              style={{ flex: s, background: scoreColor(s), minWidth: s > 0 ? 2 : 0 }}
            />
          ))}
        </div>
      )}

      {showDetail && (
        <div className="exec-task-detail">
          {leaf.completions.map((comp, ci) => (
            <AgentCompletionChat
              key={comp.chunk.id || ci}
              chunk={comp.chunk}
              error={comp.error}
              id={comp.chunk.id}
            />
          ))}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

export function FunctionExecutionView({ entry }: { entry: FunctionExecutionEntry }) {
  const chunk = entry.chunk;
  const topError = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const leaves = chunk ? collectLeaves(chunk) : [];
  const rawOutput = chunk?.output ?? null;
  const isVector = Array.isArray(rawOutput);
  const output = rawOutput as number | number[] | null;
  const hasOutput = output != null;

  const state = topError
    ? "error"
    : hasOutput
      ? "complete"
      : leaves.length > 0
        ? "streaming"
        : "pending";

  return (
    <div className="exec-container">
      <div className="exec-header">
        <span className="exec-status" style={{ background: stateColor(state) }} />
        <span className="exec-title">Function Execution</span>
        {hasOutput && (
          <span className="exec-output">
            {isVector
              ? `#${(output as number[]).indexOf(Math.max(...(output as number[]))) + 1} · ${pct(Math.max(...(output as number[])))}`
              : pct(output as number)}
          </span>
        )}
        <span className="exec-header-id">{entry.id.slice(0, 12)}</span>
      </div>

      {isVector && (
        <div className="exec-output-bar">
          {(output as number[]).map((s, i) => (
            <div
              key={i}
              className="exec-output-seg"
              style={{ flex: s, background: scoreColor(s), minWidth: s > 0 ? 2 : 0 }}
            />
          ))}
        </div>
      )}

      <div className="exec-body">
        {leaves.map((leaf) => {
          if (leaf.type === "reasoning") {
            return (
              <AgentCompletionChat
                key={leaf.data.key}
                label={leaf.data.label}
                chunk={leaf.data.chunk}
                error={leaf.data.error}
                id={leaf.data.chunk.id}
              />
            );
          }
          return <TaskCard key={leaf.data.key} leaf={leaf.data} />;
        })}

        {leaves.length === 0 && !topError && (
          <div className="viewer-empty">Waiting for execution...</div>
        )}
      </div>

      {topError && (
        <div className="ac-error-banner">
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}

      {chunk?.usage && (
        <div className="ac-footer">
          <div className="ac-footer-item">
            <span className="ac-footer-label">Prompt:</span>
            <span>{chunk.usage.prompt_tokens}</span>
          </div>
          <div className="ac-footer-item">
            <span className="ac-footer-label">Completion:</span>
            <span>{chunk.usage.completion_tokens}</span>
          </div>
          <div className="ac-footer-item">
            <span className="ac-footer-label">Total:</span>
            <span>{chunk.usage.total_tokens}</span>
          </div>
          {chunk.usage.cost !== undefined && chunk.usage.cost !== 0 && (
            <div className="ac-footer-item">
              <span className="ac-footer-label">Cost:</span>
              <span>${typeof chunk.usage.cost === "number" ? chunk.usage.cost.toFixed(6) : chunk.usage.cost}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
