// A log is pieces, not messages: text arrives a few words at a time, a
// tool call and its answer are paired by id, and anything carrying a
// `parent` belongs inside that tool call (a helper's own conversation).
// This folds the pieces into what a person reads. Pure — tested.

import type { LogEntry } from "../bindings/LogEntry";
import type { ProviderView } from "../bindings/ProviderView";

export type Part =
  | { kind: "reasoning"; text: string; at?: string }
  | { kind: "text"; text: string }
  | { kind: "refusal"; text: string }
  | { kind: "image"; mime: string; data: string }
  | { kind: "audio"; mime: string }
  | { kind: "tool"; id: string; name: string; arguments: string | null; answer: { text: string; isError: boolean; at?: string } | null; parts: Part[]; at?: string };

export type Block =
  | { kind: "user"; key: string; text: string; at: string; attachments: string[] }
  | { kind: "turn"; at: string; parts: Part[]; usage: { prompt: number; completion: number; total: number } | null }
  | { kind: "started"; provider: ProviderView; at: string }
  | { kind: "finished"; provider: ProviderView; at: string }
  | { kind: "error"; message: string; at: string }
  | { kind: "notice"; fatal: boolean; message: unknown; at: string };

function appendText(parts: Part[], kind: "reasoning" | "text" | "refusal", text: string, at?: string) {
  const last = parts[parts.length - 1];
  if (last && last.kind === kind) last.text += text;
  else parts.push(kind === "reasoning" ? { kind, text, at } : { kind, text });
}

export function fold(entries: LogEntry[]): Block[] {
  const blocks: Block[] = [];
  const tools = new Map<string, Extract<Part, { kind: "tool" }>>();

  const turn = (at: string): Extract<Block, { kind: "turn" }> => {
    const last = blocks[blocks.length - 1];
    if (last && last.kind === "turn") return last;
    const fresh: Extract<Block, { kind: "turn" }> = { kind: "turn", at, parts: [], usage: null };
    blocks.push(fresh);
    return fresh;
  };
  const home = (parent: string | null, at: string): Part[] => {
    if (parent) {
      const tool = tools.get(parent);
      if (tool) return tool.parts;
    }
    return turn(at).parts;
  };

  for (const { created: at, item } of entries) {
    // Tool ids are only unique within one run: a new run starts a new map.
    if (item.kind === "active") tools.clear();
    switch (item.kind) {
      case "user_text": {
        const last = blocks[blocks.length - 1];
        if (last && last.kind === "user" && last.key === item.key) last.text += item.text;
        else blocks.push({ kind: "user", key: item.key, text: item.text, at, attachments: [] });
        break;
      }
      case "user_image":
      case "user_audio":
      case "user_resource":
      case "user_resource_link": {
        const label = item.kind === "user_image" ? "image" : item.kind === "user_audio" ? "audio" : "uri" in item ? item.uri : "file";
        const last = blocks[blocks.length - 1];
        if (last && last.kind === "user" && last.key === item.key) last.attachments.push(label);
        else blocks.push({ kind: "user", key: item.key, text: "", at, attachments: [label] });
        break;
      }
      case "reasoning":
        appendText(home(item.parent, at), "reasoning", item.text, at);
        break;
      case "text":
        appendText(home(item.parent, at), "text", item.text);
        break;
      case "refusal":
        appendText(home(item.parent, at), "refusal", item.text);
        break;
      case "image":
        home(item.parent, at).push({ kind: "image", mime: item.mime_type, data: item.data });
        break;
      case "audio":
        home(item.parent, at).push({ kind: "audio", mime: item.mime_type });
        break;
      case "tool_call": {
        const existing = tools.get(item.id);
        if (existing) {
          // A call can arrive in pieces too: the arguments keep coming.
          existing.arguments = (existing.arguments ?? "") + (item.arguments ?? "");
          break;
        }
        const tool: Extract<Part, { kind: "tool" }> = { kind: "tool", id: item.id, name: item.name, arguments: item.arguments, answer: null, parts: [], at };
        tools.set(item.id, tool);
        home(item.parent, at).push(tool);
        break;
      }
      case "tool_response": {
        const tool = tools.get(item.id);
        if (tool) tool.answer = { text: item.text, isError: item.is_error, at };
        else home(item.parent, at).push({ kind: "tool", id: item.id, name: "?", arguments: null, answer: { text: item.text, isError: item.is_error }, parts: [] });
        break;
      }
      case "usage": {
        const last = blocks[blocks.length - 1];
        const u = { prompt: item.prompt_tokens, completion: item.completion_tokens, total: item.total_tokens };
        if (last && last.kind === "turn") last.usage = u;
        else blocks.push({ kind: "turn", at, parts: [], usage: u });
        break;
      }
      case "notification":
        blocks.push({ kind: "notice", fatal: item.fatal, message: item.message, at });
        break;
      case "error":
        blocks.push({ kind: "error", message: item.message, at });
        break;
      case "active":
        blocks.push({ kind: "started", provider: item.provider, at });
        break;
      case "inactive":
        blocks.push({ kind: "finished", provider: item.provider, at });
        break;
      default: {
        const never: never = item;
        return never;
      }
    }
  }
  return blocks;
}

/** A one-line summary of a tool call's arguments. */
export function argsLine(args: string | null): string {
  if (!args) return "";
  try {
    const value = JSON.parse(args);
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const first = Object.values(value)[0];
      if (typeof first === "string") return first;
      if (Array.isArray(first)) return first.join(" ");
    }
    return JSON.stringify(value);
  } catch {
    return args;
  }
}

export function pretty(args: string | null): string {
  if (!args) return "";
  try {
    return JSON.stringify(JSON.parse(args), null, 2);
  } catch {
    return args;
  }
}

// --- turns that read ---------------------------------------------------------
// What a person reads is what the agent SAID. Everything it did between two
// things it said (thinking, tool calls, helpers) folds into one line of work.

export type Shown =
  | { kind: "say"; part: Part }
  | { kind: "work"; steps: Part[]; summary: WorkSummary };

export type WorkSummary = {
  tools: { name: string; count: number }[];
  thought: boolean;
  helpers: number;
  failed: number;
  waiting: boolean;
  seconds: number | null;
  last: Extract<Part, { kind: "tool" }> | null;
};

function times(p: Part): string[] {
  if (p.kind === "reasoning") return p.at ? [p.at] : [];
  if (p.kind === "tool") return [p.at, p.answer?.at, ...p.parts.flatMap(times)].filter((x): x is string => !!x);
  return [];
}

export function summarize(steps: Part[]): WorkSummary {
  const counts = new Map<string, number>();
  let thought = false, helpers = 0, failed = 0, waiting = false;
  let last: Extract<Part, { kind: "tool" }> | null = null;
  for (const p of steps) {
    if (p.kind === "reasoning") thought = true;
    if (p.kind === "tool") {
      counts.set(p.name, (counts.get(p.name) ?? 0) + 1);
      if (p.parts.length > 0) helpers++;
      if (p.answer?.isError) failed++;
      if (!p.answer) waiting = true;
      last = p;
    }
  }
  const stamps = steps.flatMap(times).map((t) => Date.parse(t)).filter((n) => !Number.isNaN(n));
  const seconds = stamps.length > 1 ? Math.max(0, Math.round((Math.max(...stamps) - Math.min(...stamps)) / 1000)) : null;
  return { tools: [...counts].map(([name, count]) => ({ name, count })), thought, helpers, failed, waiting, seconds, last };
}

export function shown(parts: Part[]): Shown[] {
  const out: Shown[] = [];
  let work: Part[] = [];
  const flush = () => {
    if (work.length) out.push({ kind: "work", steps: work, summary: summarize(work) });
    work = [];
  };
  for (const p of parts) {
    if (p.kind === "reasoning" || p.kind === "tool") work.push(p);
    else {
      flush();
      out.push({ kind: "say", part: p });
    }
  }
  flush();
  return out;
}
