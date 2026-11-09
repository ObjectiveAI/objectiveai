// The world's lists kept open: each entity's live state as it changes,
// and every volume as it is listed.

import { daemon, shared } from "@diverge/sdk";
import { handle } from "../lib/wire";
import { type Live, accountKey, accountLive, activeLive, agentKey, connectedKey, daemonKey, daemonLive, dependencyKey, providerKey, providerLive, toolKey } from "./model";

/** One volume, as the volumes list states it. */
export type Volume = daemon.endpoints.volumes.list.server.response.Volume;

/** How long, in milliseconds, before a list that ended is opened again. */
const AGAIN = 5000;

/**
 * What is told of one entity: its key, and its live state, or `null`
 * when it is gone; and, for an agent or a tool, whether its container
 * is up — for an account, a provider or a remote daemon, whether it is
 * connected.
 */
type Told = (key: string, live: Live | null, deployed?: boolean) => void;

/** One list kept open: how it is opened and how a frame is read. */
export type Opened<T, F> = {
  open: () => Promise<[AsyncIterable<F>, { cancel(): Promise<void> }]>;
  change: (frame: F) => shared.listing.Change<T> | null;
};

/** One list kept open, and how an item is told. */
type Spec<T, F> = Opened<T, F> & {
  key: (item: T) => string;
  live: (item: T) => Live;
  deployed?: (item: T) => boolean;
};

/** Lists being watched, until `stop`. */
export type Watching = { stop(): void };

/** What a set of kept lists share: whether they are stopped, and how each open one is cancelled. */
export type Keeper = { stopped: () => boolean; cancels: Set<{ cancel(): Promise<void> }> };

/** A keeper, and the `Watching` that stops every list kept with it. */
export function keeper(): Keeper & Watching {
  let stopped = false;
  const cancels = new Set<{ cancel(): Promise<void> }>();
  return {
    stopped: () => stopped,
    cancels,
    stop() {
      stopped = true;
      for (const cancel of cancels) void cancel.cancel();
    },
  };
}

/**
 * Keep one list open and hand on every change it reads.
 *
 * The rules:
 *
 * 1. Every change is handed on, `listed` included, in the order read.
 * 2. A list that ends, fails or is refused is opened again `AGAIN`
 *    milliseconds later, until the keeper stops.
 */
export function keep<T, F>(spec: Opened<T, F>, each: (change: shared.listing.Change<T>) => void, k: Keeper): void {
  void (async () => {
    while (!k.stopped()) {
      try {
        const [stream, cancel] = await spec.open();
        k.cancels.add(cancel);
        for await (const frame of stream) {
          if (k.stopped()) break;
          const change = spec.change(frame);
          if (change !== null) each(change);
        }
        k.cancels.delete(cancel);
      } catch {
        // Rule 2: tried again below.
      }
      if (!k.stopped()) await new Promise((wake) => setTimeout(wake, AGAIN));
    }
  })();
}

/**
 * Keep one list open and tell every item's state: every item added or
 * changed with its state, one removed with `null`. The items before
 * `listed` are told as well, so a state that changed between the
 * snapshot and the watch is not lost.
 */
function tell<T, F>(spec: Spec<T, F>, told: Told, k: Keeper): void {
  keep(
    spec,
    (change) => {
      if (change.kind === "listed") return;
      if (change.kind === "removed") return told(spec.key(change.item), null);
      told(spec.key(change.item), spec.live(change.item), spec.deployed?.(change.item));
    },
    k,
  );
}

/**
 * Watch the accounts, agents, standalone tools, connected standalone
 * tools, dependency tools, providers and remote daemons,
 * telling each entity's state as it changes; and the volumes, telling
 * `volume` each one listed, added or changed, and each one removed
 * (`gone`).
 */
export function watch(told: Told, volume: (item: Volume, gone: boolean) => void): Watching {
  const k = keeper();
  const e = daemon.endpoints;
  const deployed = (x: { deployed: boolean }) => x.deployed;
  tell({ open: () => e.accounts.list.client.execute(handle, {}), change: e.accounts.list.server.response.change, key: accountKey, live: accountLive, deployed: (a) => a.connected }, told, k);
  tell({ open: () => e.agents.list.client.execute(handle, {}), change: e.agents.list.server.response.change, key: agentKey, live: activeLive, deployed }, told, k);
  tell({ open: () => e.standalone_tools.list.client.execute(handle, {}), change: e.standalone_tools.list.server.response.change, key: toolKey, live: activeLive, deployed }, told, k);
  // A connected tool is deployed while the daemon holds the other daemon's connect on it.
  tell({ open: () => e.connected_standalone_tools.list.client.execute(handle, {}), change: e.connected_standalone_tools.list.server.response.change, key: connectedKey, live: activeLive, deployed }, told, k);
  tell({ open: () => e.dependency_tools.list.client.execute(handle, {}), change: e.dependency_tools.list.server.response.change, key: dependencyKey, live: activeLive, deployed }, told, k);
  // A provider is lit while the daemon holds a connection it uses: `connected`, and nothing else.
  tell({ open: () => e.providers.list.client.execute(handle, {}), change: e.providers.list.server.response.change, key: providerKey, live: providerLive, deployed: (p) => providerLive(p).on }, told, k);
  // A remote daemon is lit while the daemon holds a connection to it.
  tell({ open: () => e.daemons.list.client.execute(handle, {}), change: e.daemons.list.server.response.change, key: daemonKey, live: daemonLive, deployed: (d) => d.connected }, told, k);
  keep({ open: () => e.volumes.list.client.execute(handle, {}), change: e.volumes.list.server.response.change }, (change) => {
    if (change.kind !== "listed") volume(change.item, change.kind === "removed");
  }, k);
  return k;
}
