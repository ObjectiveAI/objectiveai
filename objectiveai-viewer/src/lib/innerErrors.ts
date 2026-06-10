import type { Entry } from "../types";

export interface InnerError {
  path: string;
  code: number;
  message: string;
}

function walkError(obj: unknown, path: string, out: InnerError[]): void {
  if (!obj || typeof obj !== "object") return;
  const o = obj as Record<string, unknown>;

  if (o.error && typeof o.error === "object") {
    const err = o.error as { code?: number; message?: unknown };
    out.push({
      path,
      code: err.code ?? 0,
      message: String(err.message ?? "Unknown error"),
    });
  }

  if (Array.isArray(o.tasks)) {
    for (let i = 0; i < o.tasks.length; i++) {
      const task = o.tasks[i] as Record<string, unknown> | null;
      if (!task) continue;
      const taskPath = Array.isArray(task.task_path) && task.task_path.length > 0
        ? (task.task_path as number[]).join(".")
        : `${path}/${i}`;
      walkError(task, taskPath, out);

      if (Array.isArray(task.completions)) {
        for (let ci = 0; ci < task.completions.length; ci++) {
          walkError(task.completions[ci], `${taskPath}/comp[${ci}]`, out);
        }
      }
    }
  }

  if (Array.isArray(o.completions)) {
    for (let i = 0; i < o.completions.length; i++) {
      walkError(o.completions[i], `${path}/comp[${i}]`, out);
    }
  }

  if (o.reasoning && typeof o.reasoning === "object") {
    walkError(o.reasoning, `${path}/reasoning`, out);
  }
}

export function collectInnerErrors(entry: Entry): InnerError[] {
  if (!entry.chunk) return [];
  const out: InnerError[] = [];
  walkError(entry.chunk, "root", out);
  return out;
}

export function countInnerErrors(entry: Entry): number {
  return collectInnerErrors(entry).length;
}
