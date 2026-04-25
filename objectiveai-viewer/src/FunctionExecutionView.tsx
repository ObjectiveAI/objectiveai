import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "objectiveai";
import { AgentCompletionChat } from "./AgentCompletionView";

interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: { id: string };
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

// ---------------------------------------------------------------------------
// Tree walk
// ---------------------------------------------------------------------------
//
// A function execution is a tree:
//
//   FunctionExecutionChunk (root)
//   ├── reasoning?  ← one agent completion (its own response id)
//   └── tasks[]
//       ├── FunctionExecutionTaskChunk  (a wrapped sub-execution)
//       │   ├── reasoning?
//       │   └── tasks[]   ← recurses, possibly many levels deep
//       │       (split mode wraps an entire execution per array element;
//       │        swiss strategy wraps an execution per pool per round;
//       │        nested branch functions wrap their sub-functions; etc.)
//       └── VectorCompletionTaskChunk   (a leaf)
//           └── completions[]   ← one or more agent completions, each with a
//                                 unique response id
//
// We exhaustively walk every nested wrapper, picking up every reasoning
// summary and every vector-completion's agent completions as we go.

/// One renderable chat entry: a single agent-completion chunk plus the
/// path / modifiers that locate it within the execution tree.
interface ChatLeaf {
  /// React key — the inner completion's unique response_id. The execution
  /// tree assigns a fresh response_id to each individual agent completion,
  /// so this is sufficient.
  key: string;
  /// Human-readable label shown above the chat.
  label: string;
  /// The actual agent-completion chunk (vector-completion variant has the
  /// same flat shape as the regular agent-completion chunk).
  chunk: VectorCompletionsResponseStreamingAgentCompletionChunk;
  /// Optional per-completion error.
  error: { code: number; message: unknown } | null;
}

// `TaskChunk` is `#[serde(untagged)]` in Rust, so the JSON variants are
// distinguished only by structure. The single most reliable discriminator
// is the `object` marker every inner chunk carries.
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

function formatLabel(path: number[], modifiers: string[], suffix: string): string {
  const p = path.length === 0 ? "(root)" : path.join(".");
  const base = modifiers.length === 0 ? p : `${p}  [${modifiers.join(", ")}]`;
  return suffix ? `${base} — ${suffix}` : base;
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

function emitReasoning(
  reasoning: FunctionsExecutionsResponseStreamingReasoningSummaryChunk | null | undefined,
  path: number[],
  modifiers: string[],
  out: ChatLeaf[],
): void {
  if (!reasoning) return;
  // ReasoningSummaryChunk = AgentCompletionChunk fields (flattened) + error.
  // The chunk itself satisfies the AgentCompletionChat shape directly.
  const r = reasoning as unknown as VectorCompletionsResponseStreamingAgentCompletionChunk & {
    error?: { code: number; message: unknown } | null;
  };
  const compError = reasoning.error
    ? { code: reasoning.error.code, message: reasoning.error.message }
    : null;
  out.push({
    key: `reasoning-${r.id || `${path.join(".")}-${modifiers.join(",")}`}`,
    label: formatLabel(path, modifiers, "reasoning"),
    chunk: r,
    error: compError,
  });
}

function walkTasks(
  tasks: FunctionsExecutionsResponseStreamingTaskChunk[] | undefined,
  inheritedModifiers: string[],
  out: ChatLeaf[],
): void {
  if (!tasks) return;
  for (const t of tasks) {
    if (isVectorCompletionTask(t)) {
      const v = t as unknown as FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk;
      const path = v.task_path ?? [];
      const completions = v.completions ?? [];
      for (let ci = 0; ci < completions.length; ci++) {
        const comp = completions[ci];
        const compError = comp.error
          ? { code: comp.error.code, message: comp.error.message }
          : null;
        out.push({
          key: `comp-${comp.id || `${path.join(".")}-${comp.index ?? ci}`}`,
          label: formatLabel(path, inheritedModifiers, `completion #${comp.index ?? ci}`),
          chunk: comp,
          error: compError,
        });
      }
      continue;
    }
    if (isFunctionExecutionTask(t)) {
      const fe = t as unknown as FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk;
      const mods = [...inheritedModifiers, ...modifiersFor(fe)];
      // Reasoning at THIS nesting level (its own agent completion).
      emitReasoning(fe.reasoning, fe.task_path ?? [], mods, out);
      // Recurse into the wrapper's own tasks (which may themselves contain
      // more wrappers, indefinitely deep).
      walkTasks(fe.tasks, mods, out);
      continue;
    }
    // Unknown task variant — silently skip rather than crash. Surfaces as
    // a missing entry, which is preferable to a blank screen.
  }
}

function collectChats(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): ChatLeaf[] {
  const out: ChatLeaf[] = [];
  // Root reasoning summary, if any.
  emitReasoning(chunk.reasoning, [], [], out);
  // Walk the whole tree.
  walkTasks(chunk.tasks, [], out);
  return out;
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

export function FunctionExecutionView({ entry }: { entry: FunctionExecutionEntry }) {
  const chunk = entry.chunk;
  const topError = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const chats = chunk ? collectChats(chunk) : [];

  return (
    <div>
      {chats.map((leaf) => (
        <AgentCompletionChat
          key={leaf.key}
          label={leaf.label}
          chunk={leaf.chunk}
          error={leaf.error}
          id={leaf.chunk.id}
        />
      ))}

      {chats.length === 0 && !topError && (
        <div
          style={{
            maxWidth: 800,
            margin: "0 auto 24px",
            padding: 16,
            color: "#999",
            fontStyle: "italic",
            textAlign: "center",
          }}
        >
          Waiting for execution…
        </div>
      )}

      {topError && (
        <div
          className="ac-error-banner"
          style={{
            maxWidth: 800,
            margin: "0 auto 24px",
            border: "1px solid #f5c6cb",
            borderRadius: 8,
          }}
        >
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
