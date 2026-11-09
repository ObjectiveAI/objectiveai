// The messages sent to agents and not yet taken, kept for the page's
// life whatever sheet is open.

import { daemon } from "@diverge/sdk";
import { handle, type Route, relayed } from "../lib/wire";
import type { Block } from "./attach";

/** An agent, as a request names it. */
type Agent = daemon.reference.Agent;

/** A message's answer, waited for or cancelled. */
type Pending = Awaited<ReturnType<typeof daemon.endpoints.agents.message.client.execute>>;

/**
 * One message in the outbox: its content, and whether it still waits
 * for the loop to take it or failed, and why.
 */
export type Waiting = { id: number; content: Block[]; state: "waiting" | "failed"; why?: string };

/** A message as the outbox holds it: what is shown, its answer once asked, and whether ✕ was pressed. */
type Held = Waiting & { pending: Pending | null; cancelled: boolean };

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
 * 2. `delivered` — the loop took it, and the log shows it now — and
 *    no answer at all — a cancel in time, the message never taken —
 *    take it out.
 * 3. `forbidden`, an error, and a request that fails mark it failed,
 *    with words that say why; it stays until its ✕.
 * 4. `drop` is the ✕: a waiting message is cancelled, and taken out
 *    when its answer says how the cancel went (rule 2, or rule 3); a
 *    message whose request is not yet open is cancelled as soon as it
 *    opens. A failed message is taken out at once.
 * 5. The outbox outlives every sheet: closing or swapping a sheet
 *    neither cancels nor loses a message, and the agent's sheet shows
 *    it again when opened.
 * 6. Every change is told to the outbox's listeners (`watch`), until
 *    they stop listening.
 */
export const outbox = {
  /** Rule 1. */
  send(route: Route, agent: Agent, key: string, content: Block[]): void {
    const held: Held = { id: next++, content, state: "waiting", pending: null, cancelled: false };
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
        if (answer === null || answer === "delivered") return remove(key, held);
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
    if (held.cancelled) return;
    held.cancelled = true;
    void held.pending?.cancel();
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
