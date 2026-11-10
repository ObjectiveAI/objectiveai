// The messages sent to agents and not yet in their logs, kept for the
// page's life whatever sheet is open.

import { daemon } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";
import type { Block } from "./attach";

/** An agent, as a request names it. */
type Agent = daemon.reference.Agent;

/** A message's answer, waited for or cancelled. */
type Pending = Awaited<ReturnType<typeof daemon.endpoints.agents.message.client.execute>>;

/** One item of an agent's log. */
type Item = daemon.agent_log.ItemWrapper;

/**
 * One message in the outbox: its content, and whether it still waits
 * for the loop to take it, was taken and is not yet in the log, or
 * failed, and why.
 */
export type Waiting = { id: number; content: Block[]; state: "waiting" | "taken" | "failed"; why?: string };

/** A message as the outbox holds it: what is shown, its answer once asked, whether ✕ was pressed, and whether the log showed it. */
type Held = Waiting & { pending: Pending | null; cancelled: boolean; seen: boolean };

/** Every agent's messages, under its outbox key (`keyOf`), oldest first. */
const boxes = new Map<string, Held[]>();

/** Who is told of a change to each outbox. */
const listeners = new Map<string, Set<() => void>>();

/** The next message's ID. */
let next = 0;

/** An agent's outbox, by the route to its daemon and its entity key: the same agent at Home and in a tab are two. */
export const keyOf = (route: Route, agent: string) => `${route.join("/")}\0${agent}`;

/** Tell `key`'s listeners. */
function told(key: string): void {
  for (const listener of listeners.get(key) ?? []) listener();
}

/** Take a message out of its outbox. */
function remove(key: string, held: Held): void {
  const box = boxes.get(key);
  if (!box) return;
  const at = box.indexOf(held);
  if (at < 0) return;
  box.splice(at, 1);
  if (box.length === 0) boxes.delete(key);
  told(key);
}

/** Rule 2: mark a message taken, or take it out if the log showed it already. */
function taken(key: string, held: Held): void {
  if (held.seen) return remove(key, held);
  held.state = "taken";
  held.pending = null;
  told(key);
}

/**
 * Rule 7: what a block is matched by — its text, else its data, else
 * its link, else its JSON. A message is matched by its first text
 * block, else by its first block; a log's user part by its own.
 */
function mark(block: object): string {
  if ("text" in block) return `text:${String(block.text)}`;
  if ("data" in block) return `data:${String(block.data)}`;
  if ("uri" in block) return `uri:${String(block.uri)}`;
  if ("resource" in block) return `resource:${JSON.stringify(block.resource)}`;
  return JSON.stringify(block);
}

/** Mark a message failed, in words. */
function fail(key: string, held: Held, why: string): void {
  held.state = "failed";
  held.why = why;
  held.pending = null;
  told(key);
}

/** A failure's value, in words: a string as it is, anything else as JSON. */
const words = (error: unknown) => (typeof error === "string" ? error : JSON.stringify(error));

/**
 * The outbox.
 *
 * The rules:
 *
 * 1. `send` puts a message in its agent's outbox, waiting, and asks
 *    the daemon the route reaches to hand it over (`agents message`).
 * 2. `delivered` — the loop took it — marks it taken: it stays, with
 *    no ✕, until the log shows it (rule 7), and is taken out at once
 *    if the log showed it already. No answer at all — a cancel in
 *    time, the message never taken — takes it out.
 * 3. `forbidden`, an error, and a request that fails mark it failed,
 *    with words that say why; it stays until its ✕.
 * 4. `drop` is the ✕: a waiting message is cancelled, and taken out
 *    when its answer says how the cancel went (rule 2, or rule 3); a
 *    message whose request is not yet open is cancelled as soon as it
 *    opens. A failed message is taken out at once; a taken one has no
 *    ✕, and is left.
 * 5. The outbox outlives every sheet: closing or swapping a sheet
 *    neither cancels nor loses a message, and the agent's sheet shows
 *    it again when opened.
 * 6. Every change is told to the outbox's listeners (`watch`), until
 *    they stop listening.
 * 7. `seen` is the agent's log, as the open sheet follows it, history
 *    and all: a user part an account sent — the world does not know
 *    which account it is, so any account's — shows the oldest message
 *    in the outbox, waiting or taken and not yet shown, whose `mark`
 *    is the part's. One part shows one message. A taken message shown
 *    is taken out; a waiting one is marked shown, for rule 2. A
 *    failed message is never shown.
 */
export const outbox = {
  /** Rule 1. */
  send(route: Route, agent: Agent, key: string, content: Block[]): void {
    const held: Held = { id: next++, content, state: "waiting", pending: null, cancelled: false, seen: false };
    const box = boxes.get(key) ?? [];
    box.push(held);
    boxes.set(key, box);
    told(key);
    void (async () => {
      try {
        const pending = await daemon.endpoints.agents.message.client.execute(handle, { agent, content, ...relayed(route) });
        held.pending = pending;
        // Rule 4: pressed before the request opened.
        if (held.cancelled) void pending.cancel();
        const answer = await pending.wait();
        // Rule 2.
        if (answer === "delivered") return taken(key, held);
        if (answer === null) return remove(key, held);
        // Rule 3.
        if (answer === "forbidden") return fail(key, held, "this account may not message the agent");
        fail(key, held, `the daemon could not deliver it: ${words(answer.error)}`);
      } catch (error) {
        fail(key, held, error instanceof Error ? error.message : String(error));
      }
    })();
  },

  /** Rule 4. */
  drop(key: string, id: number): void {
    const held = boxes.get(key)?.find((m) => m.id === id);
    if (!held) return;
    if (held.state === "failed") return remove(key, held);
    if (held.state === "taken" || held.cancelled) return;
    held.cancelled = true;
    void held.pending?.cancel();
  },

  /** Rule 7. */
  seen(key: string, item: Item): void {
    if (!("sender" in item && "key" in item && item.type.startsWith("user_"))) return;
    if ((item.sender as daemon.creator.Creator).type !== "client") return;
    const part = mark(item);
    const held = boxes.get(key)?.find((m) => m.state !== "failed" && !m.seen && mark(m.content.find((b) => b.type === "text") ?? m.content[0] ?? {}) === part);
    if (!held) return;
    if (held.state === "taken") return remove(key, held);
    held.seen = true;
  },

  /** The messages in an outbox, oldest first. */
  of(key: string): readonly Waiting[] {
    return boxes.get(key) ?? [];
  },

  /** Rule 6: `listener` is told of every change to `key`'s outbox until the function returned is called. */
  watch(key: string, listener: () => void): () => void {
    const set = listeners.get(key) ?? new Set();
    set.add(listener);
    listeners.set(key, set);
    return () => {
      set.delete(listener);
      if (set.size === 0) listeners.delete(key);
    };
  },
};
