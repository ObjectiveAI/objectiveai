import type { TaskDefinition } from "@/lib/functions/types";
import type { ProfileMeta, ProfileLlm } from "@/lib/profiles/types";

/* ── Execution types (SDK streaming shape) ── */

export interface Vote {
  model: string;
  vote: number[];
  weight: number;
  from_cache?: boolean;
  from_rng?: boolean;
}

export interface TaskExecution {
  task_path?: number[];
  votes?: Vote[];
  completions?: Array<Record<string, unknown>>;
  scores?: number[];
  error?: { message?: string } | null;
  tasks?: TaskExecution[];
  output?: number | number[];
}

export interface FunctionExecution {
  id?: string;
  function?: string;
  profile?: string;
  output?: number | number[];
  error?: unknown;
  reasoning?: { choices?: Array<{ message?: { content?: string } }> };
  tasks?: TaskExecution[];
}

/* ── Helpers ── */

export function scoreColor(s: number): string {
  if (s >= 0.5) return "var(--copper-hot)";
  if (s >= 0.3) return "var(--copper-mid)";
  if (s >= 0.15) return "var(--copper-warm)";
  return "var(--copper-dim)";
}

export function pct(n: number): string {
  return (n * 100).toFixed(1) + "%";
}

export function dotPct(n: number): string {
  return "." + (n * 100).toFixed(0).padStart(2, "0");
}

export function stateColor(s: string): string {
  if (s === "complete") return "var(--copper-hot)";
  if (s === "streaming") return "var(--copper-mid)";
  if (s === "error") return "var(--error)";
  return "var(--node-border)";
}

export function taskState(exec?: TaskExecution | null): string {
  if (!exec) return "structural";
  if (exec.error) return "error";
  if (exec.scores?.length) return "complete";
  if (exec.completions?.length) return "streaming";
  return "pending";
}

export function funcState(exec?: FunctionExecution | null): string {
  if (!exec) return "structural";
  if (exec.error) return "error";
  if (exec.output != null) return "complete";
  if (exec.tasks?.length) return "streaming";
  return "pending";
}

export function normalizeType(t: string): string {
  return t.replace(/^alpha\./, "");
}

export function shortType(t: string): string {
  const n = normalizeType(t);
  if (n === "vector.completion") return "vc";
  if (n.includes("placeholder")) return "placeholder";
  if (n.includes("vector.function")) return "vector fn";
  if (n.includes("scalar.function")) return "scalar fn";
  return n;
}

export function labelFor(r: unknown, i: number): string {
  if (typeof r === "string") return r.length > 24 ? r.slice(0, 22) + "\u2026" : r;
  if (r && typeof r === "object") {
    const o = r as Record<string, unknown>;
    if (typeof o.text === "string") return o.text.length > 24 ? o.text.slice(0, 22) + "\u2026" : o.text;
    if (o.$jmespath || o.$starlark) return "[dynamic]";
  }
  return `#${i + 1}`;
}

export function promptPreview(task: TaskDefinition): string | null {
  if (!Array.isArray(task.messages)) return null;
  const msgs = task.messages as Array<Record<string, unknown>>;
  for (const role of ["system", "developer", "user"]) {
    const m = msgs.find((x) => x.role === role);
    if (m && typeof m.content === "string") {
      return m.content.length > 80 ? m.content.slice(0, 77) + "\u2026" : m.content;
    }
  }
  return null;
}

export function exprStr(expr: Record<string, unknown> | undefined): string | null {
  if (!expr) return null;
  if (typeof expr.$jmespath === "string") return `jmes: ${expr.$jmespath}`;
  if (typeof expr.$starlark === "string") {
    const s = expr.$starlark as string;
    return s.length > 40 ? `star: ${s.slice(0, 37)}\u2026` : `star: ${s}`;
  }
  return null;
}

export function getTaskAgents(
  profile: ProfileMeta | null | undefined,
  ti: number,
): { llms: ProfileLlm[]; weights: number[] } | null {
  if (!profile) return null;
  if (profile.kind === "tasks" && profile.taskConfigs[ti]) return profile.taskConfigs[ti];
  return { llms: profile.llms, weights: profile.weights };
}

export function getTaskWeight(profile: ProfileMeta | null | undefined, ti: number, total: number): number | null {
  if (!profile) return null;
  if (profile.kind === "tasks" && profile.taskWeights.length > ti) return profile.taskWeights[ti];
  return total > 0 ? 1 / total : null;
}
