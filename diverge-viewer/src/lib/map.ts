// The Agents map's logic: where an arrow key goes in the tree, and what an
// agent's card says it is doing, from the tail of its log. The tree itself
// is Rust's (fleet.rs, sent with every agents snapshot). Pure — tested.

import type { LogEntry } from "../bindings/LogEntry";
import type { MapNode } from "../bindings/MapNode";
import { argsLine, fold, shown, type Part } from "./conversation";
import { t } from "../strings";

/** Which way the map grows: what an agent made hangs below it, or beside it. */
export type Layout = "down" | "across";
/** The map opens this way. One place to change it. */
export const DEFAULT_LAYOUT: Layout = "down";

/** How far back a card reads an agent's log: enough for the run it's in, or the last one. */
export const TAIL = 60;
export const tailFrom = (logsIndex: number) => Math.max(0, logsIndex - TAIL);

/** Each node's place: its parent, its siblings (itself among them) and what hangs under it. */
type Place = { parent: string | null; siblings: string[]; children: string[] };

export function places(tree: MapNode[]): { order: string[]; at: Map<string, Place> } {
  const order: string[] = [];
  const at = new Map<string, Place>();
  const walk = (nodes: MapNode[], parent: string | null) => {
    const siblings = nodes.map((n) => n.key);
    for (const n of nodes) {
      order.push(n.key);
      at.set(n.key, { parent, siblings, children: n.children.map((c) => c.key) });
      walk(n.children, n.key);
    }
  };
  walk(tree, null);
  return { order, at };
}

/** Where an arrow key moves from `from`: along the way the map grows to what it made, back to its maker,
 *  or across to the one beside it; Home and End go to the first and last. Nowhere else: null. */
export function step(tree: MapNode[], from: string | null, key: string, layout: Layout): string | null {
  const { order, at } = places(tree);
  if (order.length === 0) return null;
  const here = from !== null ? at.get(from) : undefined;
  if (key === "Home") return order[0];
  if (key === "End") return order[order.length - 1];
  if (!here || from === null) return ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(key) ? order[0] : null;
  const [deeper, back, next, previous] = layout === "down" ? ["ArrowDown", "ArrowUp", "ArrowRight", "ArrowLeft"] : ["ArrowRight", "ArrowLeft", "ArrowDown", "ArrowUp"];
  const i = here.siblings.indexOf(from);
  switch (key) {
    case deeper:
      return here.children[0] ?? null;
    case back:
      return here.parent;
    case next:
      return here.siblings[i + 1] ?? null;
    case previous:
      return here.siblings[i - 1] ?? null;
    default:
      return null;
  }
}

/** Every agent in the map, by key, wherever it hangs. */
export function agentsIn(tree: MapNode[]): string[] {
  return tree.flatMap((n) => [...(n.what.kind === "agent" ? [n.what.agent.key] : []), ...agentsIn(n.children)]);
}

// --- what a card says --------------------------------------------------------

/** A helper (a tool call with work of its own inside it) in the run a card shows. */
export type Helper = { id: string; name: string; line: string; done: boolean };

/** What a card says an agent is doing, or last did: one line, the helpers in that run, and whether that
 *  run is still going (by its own log). */
export type Doing = { line: string | null; helpers: Helper[]; running: boolean };

/** Words as one plain line: Markdown marks and line breaks dropped. */
function plain(text: string): string {
  return text.replace(/[*_`#>]+/g, "").replace(/\s+/g, " ").trim();
}

/** The latest thing in some work, as a line: a call (and what it was asked), a thought, or words. */
function latest(parts: Part[]): string | null {
  for (let i = parts.length - 1; i >= 0; i--) {
    const p = parts[i];
    switch (p.kind) {
      case "tool": {
        const inner = !p.answer && p.parts.length ? latest(p.parts) : null;
        if (inner) return inner;
        const args = argsLine(p.arguments);
        return `${t.convo.doorTools[p.name] ?? p.name}${args ? `: ${plain(args)}` : ""}`;
      }
      case "reasoning":
      case "text":
      case "refusal": {
        const words = plain(p.text);
        if (words) return words;
        break;
      }
      case "image":
        return t.convo.image;
      case "audio":
        return t.convo.audio;
    }
  }
  return null;
}

/** What a card shows, from the tail of an agent's log: the run it's in (or its last one), its latest line
 *  — the work going on now, or what it last said — and the helpers that run has. Whether it's working is
 *  the live list's word (`active`): a tail read before a run ended doesn't keep it going. */
export function doing(entries: LogEntry[], active: boolean): Doing {
  const blocks = fold(entries);
  let start = -1;
  for (let i = blocks.length - 1; i >= 0; i--) {
    if (blocks[i].kind === "started") {
      start = i;
      break;
    }
  }
  const run = blocks.slice(start + 1);
  const running = active && start >= 0 && !run.some((b) => b.kind === "finished");
  const parts = run.flatMap((b) => (b.kind === "turn" ? b.parts : []));
  const helpers: Helper[] = parts.flatMap((p) => (p.kind === "tool" && p.parts.length > 0 ? [{ id: p.id, name: p.name, line: latest(p.parts) ?? "", done: !!p.answer }] : []));
  // Working, it's the latest step; finished, it's what it said last (or, if it went wrong, that).
  let line: string | null = null;
  if (running) line = latest(parts);
  else {
    const last = [...run].reverse().find((b) => b.kind === "error" || b.kind === "turn");
    if (last?.kind === "error") line = `${t.common.error}: ${plain(last.message)}`;
    else if (last?.kind === "turn") {
      const said = shown(last.parts).filter((v) => v.kind === "say");
      const words = said.length ? said[said.length - 1] : null;
      line = words && words.kind === "say" ? latest([words.part]) : latest(last.parts);
    }
  }
  return { line, helpers, running };
}
