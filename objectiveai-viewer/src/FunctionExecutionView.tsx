import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "objectiveai";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";

interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: { id: string };
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

interface ChatLeaf {
  key: string;
  label: string;
  chunk: VectorCompletionsResponseStreamingAgentCompletionChunk;
  error: { code: number; message: unknown } | null;
}

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
      emitReasoning(fe.reasoning, fe.task_path ?? [], mods, out);
      walkTasks(fe.tasks, mods, out);
      continue;
    }
  }
}

function collectChats(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): ChatLeaf[] {
  const out: ChatLeaf[] = [];
  emitReasoning(chunk.reasoning, [], [], out);
  walkTasks(chunk.tasks, [], out);
  return out;
}

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
        <div className="max-w-[800px] mx-auto mb-6 p-4 text-info-dim italic text-center">
          Waiting for execution…
        </div>
      )}

      {topError && (
        <div className="max-w-[800px] mx-auto mb-6 bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs">
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
