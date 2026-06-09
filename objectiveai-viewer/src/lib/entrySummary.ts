import type { Entry } from "../types";
import { isAssistantMessage, isLastAssistantDone } from "./typeGuards";

export interface EntrySummary {
  status: "streaming" | "complete" | "error";
  kindLabel: string;
  title: string;
  detail: string;
  tokens: number;
  messageCount: number;
}

function remotePath(path: unknown): string | null {
  if (!path || typeof path !== "object") return null;
  const p = path as { owner?: string; repository?: string };
  if (p.owner && p.repository) return `${p.owner}/${p.repository}`;
  return null;
}

function extractUsage(entry: Entry): { tokens: number; messageCount: number } {
  const chunk = entry.chunk as { usage?: { total_tokens?: number }; messages?: unknown[] } | null;
  const tokens = chunk?.usage?.total_tokens ?? 0;
  const messageCount = Array.isArray(chunk?.messages) ? chunk.messages.length : 0;
  return { tokens, messageCount };
}

export function getEntrySummary(entry: Entry): EntrySummary {
  const { tokens, messageCount } = extractUsage(entry);

  if (entry.error) {
    return { status: "error", ...summarizeByKind(entry), detail: `error ${entry.error.code}`, tokens, messageCount };
  }

  const base = summarizeByKind(entry);

  if (!entry.chunk) {
    return { status: "streaming", ...base, tokens, messageCount };
  }

  switch (entry.kind) {
    case "agent-completion": {
      const done = isLastAssistantDone(entry.chunk.messages);
      return { status: done ? "complete" : "streaming", ...base, tokens, messageCount };
    }
    case "execution": {
      const done = entry.chunk.output != null;
      return { status: done ? "complete" : "streaming", ...base, tokens, messageCount };
    }
    case "invention": {
      const allDone = entry.chunk.inventions.length > 0 &&
        entry.chunk.inventions.every((inv: { function?: unknown; error?: unknown }) => inv.function != null || inv.error != null);
      return { status: allDone ? "complete" : "streaming", ...base, tokens, messageCount };
    }
    case "laboratory": {
      const done = entry.chunk.usage != null;
      return { status: done ? "complete" : "streaming", ...base, tokens, messageCount };
    }
  }
}

function summarizeByKind(entry: Entry): { kindLabel: string; title: string; detail: string } {
  switch (entry.kind) {
    case "agent-completion": {
      let model = "";
      if (entry.chunk) {
        for (const msg of entry.chunk.messages) {
          if (isAssistantMessage(msg) && msg.model) { model = msg.model; break; }
        }
      }
      return { kindLabel: "Agent", title: model || "Agent Completion", detail: "" };
    }
    case "execution": {
      const fn = entry.chunk ? remotePath(entry.chunk.function) : null;
      const taskCount = entry.chunk?.tasks?.length ?? 0;
      return {
        kindLabel: "Execution",
        title: fn || "Function",
        detail: taskCount > 0 ? `${taskCount} tasks` : "",
      };
    }
    case "invention": {
      const count = entry.chunk?.inventions?.length ?? 0;
      const firstName = entry.chunk?.inventions?.[0];
      const name = firstName && typeof firstName === "object" && "state" in firstName
        ? ((firstName as { state?: { name?: string } }).state?.name ?? null)
        : null;
      return {
        kindLabel: "Invention",
        title: name || "Invention",
        detail: count > 0 ? `${count} inventions` : "",
      };
    }
    case "laboratory": {
      const builders = entry.chunk?.builders?.length ?? 0;
      const evaluations = entry.chunk?.evaluations?.length ?? 0;
      return {
        kindLabel: "Laboratory",
        title: "Laboratory",
        detail: [
          builders > 0 ? `${builders} builders` : "",
          evaluations > 0 ? `${evaluations} evals` : "",
        ].filter(Boolean).join(", "),
      };
    }
  }
}
