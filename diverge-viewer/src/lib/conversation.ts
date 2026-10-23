// An agent's log, as the conversation shows it: the SDK's log items
// held open, adjacent pieces of one utterance joined.
//
// The daemon stores what a loop streamed: an assistant's text arrives
// as many items of the same type, one per chunk. The screen shows one
// turn, so consecutive items of one text type under one parent are
// joined here, the way the SDK's `push` joins chunks — the first
// item's index and time stand for the turn.

import { daemon } from "@diverge/sdk";
import { useEffect, useState } from "react";
import { errorText } from "./app";
import { handle } from "./daemon";
import { t } from "../strings";

/** An agent once and for all: its template and its index, the daemon's own key. */
export type AgentRef = { template: string; index: number };
export type LogItem = daemon.endpoints.agents.logs.server.response.ItemWrapper;
export type LogsRequest = daemon.endpoints.agents.logs.client.request.Frame;

/** One turn as shown: the log item, text joined across its pieces, and how many items it stands for. */
export type Turn = { item: LogItem; pieces: number };

/** The log as a screen holds it: the turns so far, and why it ended, if it did. A watch never ends on its own. */
export type Log = { turns: Turn[]; ended: string | null; done: boolean };

const JOINED = new Set(["assistant_text_content", "assistant_reasoning", "assistant_refusal"]);

/** Fold one item into the turns: joined onto the last when it continues it, else appended. */
export function fold(turns: Turn[], item: LogItem): Turn[] {
  const last = turns[turns.length - 1];
  const text = words(item);
  if (last && JOINED.has(item.type) && last.item.type === item.type && parent(last.item) === parent(item) && text !== null && words(last.item) !== null) {
    const joined = { ...last.item, text: words(last.item) + text } as LogItem;
    return [...turns.slice(0, -1), { item: joined, pieces: last.pieces + 1 }];
  }
  return [...turns, { item, pieces: 1 }];
}

/** The item's words, where it has them. */
function words(item: LogItem): string | null {
  return "text" in item && typeof item.text === "string" ? item.text : null;
}

/** The tool call an item sits under, where it sits under one. */
function parent(item: LogItem): string | undefined {
  return "parent_tool_call_id" in item && typeof item.parent_tool_call_id === "string" ? item.parent_tool_call_id : undefined;
}

/** Why a frame that is no item ended the log, as text. */
function endedBy(frame: daemon.endpoints.agents.logs.server.response.Frame): string {
  return frame === "forbidden" ? t.daemon.forbidden : JSON.stringify(frame);
}

/**
 * The log `request` asks for, read while the component lives: opened
 * once, every item folded, cancelled on the way out. `request` must
 * be one object for the component's life (memoized), since a new one
 * reopens the log.
 */
export function useLogs(request: LogsRequest | null, enabled: boolean): Log {
  const [log, setLog] = useState<Log>({ turns: [], ended: null, done: false });
  useEffect(() => {
    setLog({ turns: [], ended: null, done: false });
    if (!enabled || request === null) return;
    let stopped = false;
    let cancel: daemon.client.Cancel | null = null;
    (async () => {
      try {
        const [stream, opened] = await daemon.endpoints.agents.logs.client.execute(handle, request);
        cancel = opened;
        for await (const frame of stream) {
          if (stopped) return;
          if (typeof frame === "object" && "item" in frame) {
            const item = frame.item;
            setLog((l) => ({ ...l, turns: fold(l.turns, item) }));
          } else {
            setLog((l) => ({ ...l, ended: endedBy(frame) }));
          }
        }
        if (!stopped) setLog((l) => ({ ...l, done: true }));
      } catch (error) {
        if (!stopped) setLog((l) => ({ ...l, ended: errorText(error) }));
      }
    })();
    return () => {
      stopped = true;
      cancel?.cancel().catch(() => {});
    };
  }, [request, enabled]);
  return log;
}

/** The words of a tool result's content: its text blocks, joined; anything else named by its type. */
export function contentText(content: readonly { type: string; text?: string }[] | undefined): string {
  if (!content) return "";
  return content.map((block) => (block.type === "text" && typeof block.text === "string" ? block.text : `[${block.type}]`)).join("\n");
}
