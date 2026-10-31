// The world's lists kept open: each entity's live state as it changes.

import { daemon, shared } from "@diverge/sdk";
import { handle } from "../lib/wire";
import { type Live, accountKey, accountLive, activeLive, agentKey, dependencyKey, toolKey } from "./model";

/** How long, in milliseconds, before a list that ended is opened again. */
const AGAIN = 5000;

/** What is told of one entity: its key, and its live state, or `null` when it is gone. */
type Told = (key: string, live: Live | null) => void;

/** One list kept open: how it is opened, how a frame is read, and how an item is told. */
type Spec<T, F> = {
  open: () => Promise<[AsyncIterable<F>, { cancel(): Promise<void> }]>;
  change: (frame: F) => shared.listing.Change<T> | null;
  key: (item: T) => string;
  live: (item: T) => Live;
};

/** Lists being watched, until `stop`. */
export type Watching = { stop(): void };

/**
 * Keep one list open and tell every item's state.
 *
 * The rules:
 *
 * 1. Every item added or changed is told with its state; one removed is
 *    told with `null`. The items before `listed` are told as well, so a
 *    state that changed between the snapshot and the watch is not lost.
 * 2. A list that ends, fails or is refused is opened again `AGAIN`
 *    milliseconds later, until `stop`.
 */
function keep<T, F>(spec: Spec<T, F>, told: Told, stopped: () => boolean, cancels: Set<{ cancel(): Promise<void> }>): void {
  void (async () => {
    while (!stopped()) {
      try {
        const [stream, cancel] = await spec.open();
        cancels.add(cancel);
        for await (const frame of stream) {
          if (stopped()) break;
          const change = spec.change(frame);
          if (change === null || change.kind === "listed") continue;
          told(spec.key(change.item), change.kind === "removed" ? null : spec.live(change.item));
        }
        cancels.delete(cancel);
      } catch {
        // Rule 2: tried again below.
      }
      if (!stopped()) await new Promise((wake) => setTimeout(wake, AGAIN));
    }
  })();
}

/** Watch the accounts, agents, standalone tools and dependency tools, telling each entity's state as it changes. */
export function watch(told: Told): Watching {
  let stopped = false;
  const cancels = new Set<{ cancel(): Promise<void> }>();
  const done = () => stopped;
  const e = daemon.endpoints;
  keep({ open: () => e.accounts.list.client.execute(handle, {}), change: e.accounts.list.server.response.change, key: accountKey, live: accountLive }, told, done, cancels);
  keep({ open: () => e.agents.list.client.execute(handle, {}), change: e.agents.list.server.response.change, key: agentKey, live: activeLive }, told, done, cancels);
  keep({ open: () => e.standalone_tools.list.client.execute(handle, {}), change: e.standalone_tools.list.server.response.change, key: toolKey, live: activeLive }, told, done, cancels);
  keep({ open: () => e.dependency_tools.list.client.execute(handle, {}), change: e.dependency_tools.list.server.response.change, key: dependencyKey, live: activeLive }, told, done, cancels);
  return {
    stop() {
      stopped = true;
      for (const cancel of cancels) void cancel.cancel();
    },
  };
}
