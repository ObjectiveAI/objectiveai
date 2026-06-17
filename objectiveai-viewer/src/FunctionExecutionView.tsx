import { useState, useMemo, useRef, useCallback } from "react";
import cn from "classnames";
import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
} from "@objectiveai/sdk";
import { FunctionTree } from "./function-tree";
import type { TreeNode } from "./function-tree";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import { InnerErrorsList } from "./components/shared/InnerErrorsList";
import { OutputBar } from "./components/shared/OutputBar";
import { toInputFunctionExecution } from "./lib/treeAdapter";
import { collectInnerErrors } from "./lib/innerErrors";
import type { FunctionExecutionEntry, FunctionExecutionCreateParams } from "./types";

interface ChatLeaf {
  key: string;
  label: string;
  chunk: VectorCompletionsResponseStreamingAgentCompletionChunk;
  error: { code: number; message: unknown } | null;
  model?: string;
  scores?: number[];
  weights?: number[];
  responseLabels?: string[];
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

/**
 * Extract response labels from the request's inline function definition.
 * Walks the task tree by task_path indices to find the vector completion task
 * and extracts literal response strings when available.
 */
function extractResponseLabels(
  request: FunctionExecutionCreateParams,
  taskPath: number[],
): string[] | undefined {
  try {
    const fn = request.function;
    // Remote function references don't have inline task definitions
    if (!fn || typeof fn === "string" || !("tasks" in fn)) return undefined;
    const tasks = (fn as { tasks?: unknown[] }).tasks;
    if (!Array.isArray(tasks) || taskPath.length === 0) return undefined;

    // The task_path points to the task index at each nesting level
    // For a simple (non-nested) function, taskPath is [taskIndex]
    const taskIndex = taskPath[taskPath.length - 1];
    const task = tasks[taskIndex];
    if (!task || typeof task !== "object") return undefined;

    const taskObj = task as Record<string, unknown>;
    // Only vector.completion tasks have responses
    if (taskObj.type !== "vector.completion") return undefined;

    const responses = taskObj.responses;
    if (!responses) return undefined;

    // responses can be an expression or a literal array
    // If it's an expression object ({$jmespath: ...} or {$starlark: ...}), we can't resolve it
    if (typeof responses === "object" && !Array.isArray(responses)) return undefined;

    // It should be an array of response items
    if (!Array.isArray(responses)) return undefined;

    const labels: string[] = [];
    for (const resp of responses) {
      if (typeof resp === "string") {
        labels.push(resp);
      } else if (typeof resp === "object" && resp !== null) {
        // Could be a rich content part or an expression
        if ("$jmespath" in resp || "$starlark" in resp) {
          // Expression item - can't resolve
          return undefined;
        }
        // Rich content with text field
        if ("text" in resp && typeof (resp as { text?: unknown }).text === "string") {
          labels.push((resp as { text: string }).text);
        } else {
          // Array of parts or unknown structure - use stringified preview
          labels.push(JSON.stringify(resp).slice(0, 60));
        }
      } else {
        labels.push(String(resp));
      }
    }
    return labels.length > 0 ? labels : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Extract a human-readable task label from the request's inline function definition.
 * Walks `request.function.tasks` by indices (same pattern as extractResponseLabels)
 * to find the task and build a descriptive string.
 */
function extractTaskLabel(
  request: FunctionExecutionCreateParams,
  taskPath: number[],
): string | undefined {
  try {
    const fn = request.function;
    if (!fn || typeof fn === "string" || !("tasks" in fn)) return undefined;
    const tasks = (fn as { tasks?: unknown[] }).tasks;
    if (!Array.isArray(tasks) || taskPath.length === 0) return undefined;

    const taskIndex = taskPath[taskPath.length - 1];
    const task = tasks[taskIndex];
    if (!task || typeof task !== "object") return undefined;

    const taskObj = task as Record<string, unknown>;
    const taskType = taskObj.type as string | undefined;
    if (!taskType) return undefined;

    if (taskType === "vector.completion" || taskType === "scalar.completion") {
      const prompt = taskObj.prompt;
      if (typeof prompt === "string" && prompt.length > 0) {
        const preview = prompt.length > 40 ? prompt.slice(0, 40) + "..." : prompt;
        return `${taskType} "${preview}"`;
      }
      return taskType;
    }

    if (taskType === "vector.function" || taskType === "scalar.function") {
      const funcRef = taskObj.function;
      if (typeof funcRef === "string") {
        return `${taskType} ${funcRef}`;
      }
      if (typeof funcRef === "object" && funcRef !== null) {
        const name = (funcRef as Record<string, unknown>).name ??
          (funcRef as Record<string, unknown>).id ??
          (funcRef as Record<string, unknown>).ref;
        if (typeof name === "string") {
          return `${taskType} ${name}`;
        }
      }
      return taskType;
    }

    return taskType;
  } catch {
    return undefined;
  }
}

function walkTasks(
  tasks: FunctionsExecutionsResponseStreamingTaskChunk[] | undefined,
  inheritedModifiers: string[],
  out: ChatLeaf[],
  request?: FunctionExecutionCreateParams,
): void {
  if (!tasks) return;
  for (const t of tasks) {
    if (isVectorCompletionTask(t)) {
      const v = t as unknown as FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk;
      const path = v.task_path ?? [];
      const completions = v.completions ?? [];
      const scores = v.scores as number[] | undefined;
      const weights = v.weights as number[] | undefined;
      const responseLabels = request ? extractResponseLabels(request, path) : undefined;
      const taskLabel = request ? extractTaskLabel(request, path) : undefined;
      for (let ci = 0; ci < completions.length; ci++) {
        const comp = completions[ci];
        const compError = comp.error
          ? { code: comp.error.code, message: comp.error.message }
          : null;
        // Extract model from the first assistant message in this completion chunk
        const messages = comp.messages as Array<{ role?: string; model?: string }> | undefined;
        const assistantMsg = messages?.find((m) => m.role === "assistant");
        const model = assistantMsg?.model;
        const compIndex = comp.index ?? ci;
        const modelSuffix = model ? ` (${model})` : "";
        const suffix = taskLabel
          ? `${taskLabel} · #${compIndex}${modelSuffix}`
          : `completion #${compIndex}${modelSuffix}`;
        out.push({
          key: `comp-${comp.id || `${path.join(".")}-${compIndex}`}`,
          label: formatLabel(path, inheritedModifiers, suffix),
          chunk: comp,
          error: compError,
          model,
          scores,
          weights,
          responseLabels,
        });
      }
      continue;
    }
    if (isFunctionExecutionTask(t)) {
      const fe = t as unknown as FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk;
      const mods = [...inheritedModifiers, ...modifiersFor(fe)];
      emitReasoning(fe.reasoning, fe.task_path ?? [], mods, out);
      walkTasks(fe.tasks, mods, out, request);
      continue;
    }
  }
}

function collectChats(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  request?: FunctionExecutionCreateParams,
): ChatLeaf[] {
  const out: ChatLeaf[] = [];
  emitReasoning(chunk.reasoning, [], [], out);
  walkTasks(chunk.tasks, [], out, request);
  return out;
}

const VIEW_TABS = [
  { value: "chat" as const, label: "Chat" },
  { value: "tree" as const, label: "Tree" },
];

export function FunctionExecutionView({ entry }: { entry: FunctionExecutionEntry }) {
  const [view, setView] = useState<"chat" | "tree">("chat");
  const [highlightedKey, setHighlightedKey] = useState<string | null>(null);
  const chatRefsMap = useRef<Map<string, HTMLDivElement>>(new Map());
  const chunk = entry.chunk;
  const topError = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const chats = chunk ? collectChats(chunk, entry.request) : [];
  const innerErrors = useMemo(() => collectInnerErrors(entry), [chunk]);
  const treeData = useMemo(
    () => (chunk ? toInputFunctionExecution(chunk) : null),
    [chunk],
  );

  const handleTreeNodeClick = useCallback((node: TreeNode) => {
    if (node.data.kind !== "vector-completion") return;
    const tp = node.data.taskPath;
    const pathStr = tp.join(".");
    const match = chats.find((c) => c.key.includes(pathStr));
    if (match) {
      setHighlightedKey(match.key);
      setView("chat");
      requestAnimationFrame(() => {
        const el = chatRefsMap.current.get(match.key);
        el?.scrollIntoView({ behavior: "smooth", block: "center" });
        setTimeout(() => setHighlightedKey(null), 2000);
      });
    }
  }, [chats]);

  return (
    <div>
      {chunk && (
        <div className={cn("max-w-content", "mx-auto", "flex", "gap-1", "px-4", "mb-3", "select-none")}>
          {VIEW_TABS.map((tab) => (
            <button
              key={tab.value}
              onClick={() => setView(tab.value)}
              className={cn(
                "px-2.5",
                "py-1",
                "rounded-sm",
                "font-mono",
                "text-[10px]",
                "transition-colors",
                view === tab.value
                  ? cn("bg-copper-warm/20", "text-copper-bright")
                  : cn("bg-ground-surface", "text-info-dim", "hover:text-info-mid"),
              )}
            >
              {tab.label}
            </button>
          ))}
        </div>
      )}

      {view === "tree" && treeData && (
        <div className={cn("max-w-[1200px]", "mx-auto", "mb-6", "px-4")}>
          <div className={cn("resize-y", "overflow-auto", "min-h-[300px]", "max-h-[80vh]")} style={{ height: 500 }}>
            <FunctionTree
              data={treeData}
              config={{ theme: "dark", transparentBg: true, animate: true }}
              height="100%"
              onNodeClick={handleTreeNodeClick}
              borderless
              className={cn("rounded-md", "border", "border-node-border", "overflow-hidden")}
            />
          </div>
        </div>
      )}

      {view === "tree" && !treeData && chunk && (
        <div className={cn("max-w-[1200px]", "mx-auto", "mb-6", "px-4")}>
          <div className={cn("h-[300px]", "rounded-md", "border", "border-node-border", "bg-ground-surface", "flex", "items-center", "justify-center")}>
            <div className={cn("flex", "items-center", "gap-2", "text-info-dim", "text-xs")}>
              <span className={cn("w-1.5", "h-1.5", "rounded-full", "bg-copper-hot", "animate-pulse")} />
              Building tree…
            </div>
          </div>
        </div>
      )}

      {view === "chat" && (
        <>
          <InnerErrorsList errors={innerErrors} />
          {chats.map((leaf, idx) => {
            const showScores = leaf.scores && leaf.scores.length > 0 &&
              (idx === 0 || chats[idx - 1].scores !== leaf.scores);
            const isHighlighted = highlightedKey === leaf.key;
            return (
              <div
                key={leaf.key}
                ref={(el) => {
                  if (el) chatRefsMap.current.set(leaf.key, el);
                  else chatRefsMap.current.delete(leaf.key);
                }}
                className={cn(isHighlighted && "ring-2", isHighlighted && "ring-copper-bright/50", "rounded-md", "transition-all", "duration-500")}
              >
                <AgentCompletionChat
                  label={leaf.label}
                  chunk={leaf.chunk}
                  error={leaf.error}
                  id={leaf.chunk.id}
                />
                {showScores && (
                  <div className={cn("max-w-content", "mx-auto", "-mt-3", "mb-6", "px-4", "py-2", "bg-ground-surface", "border", "border-t-0", "border-node-border", "rounded-b-md")}>
                    <OutputBar output={leaf.scores} labels={leaf.responseLabels} />
                  </div>
                )}
              </div>
            );
          })}

          {chunk?.output !== undefined && chunk.output !== null && (
            <div className={cn("max-w-content", "mx-auto", "mb-6", "px-4", "py-3", "bg-ground-surface", "border", "border-node-border", "rounded-md")}>
              <div className={cn("text-[10px]", "font-mono", "text-info-dim", "uppercase", "tracking-wide", "mb-2")}>Output</div>
              <OutputBar
                output={chunk.output}
                labels={chats.find((c) => c.responseLabels)?.responseLabels}
              />
            </div>
          )}

          {chats.length === 0 && !topError && (
            <div className={cn("max-w-content", "mx-auto", "mb-6", "p-4", "text-info-dim", "italic", "text-center")}>
              Waiting for execution…
            </div>
          )}
        </>
      )}

      {topError && (
        <div role="alert" className={cn("max-w-content", "mx-auto", "mb-6", "bg-error/10", "border", "border-error/30", "rounded-md", "px-4", "py-2", "text-error", "text-xs")}>
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
