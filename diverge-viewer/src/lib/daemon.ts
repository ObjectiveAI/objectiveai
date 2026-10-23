// The page's side of the daemon: the SDK's handle over the app's Rust,
// the link's two commands, and the hook that keeps a list current.
//
// Every frame crosses Rust through the four proxy commands; the page
// never holds the address or the key. A list is the SDK's own fold
// (`shared.listing.Listing`) fed by the endpoint's `change()`, held
// open until the screen that asked goes away.

import { Channel, invoke } from "@tauri-apps/api/core";
import { daemon, shared, wire } from "@diverge/sdk";
import { useEffect, useState } from "react";
import type { WireStatus } from "../bindings/WireStatus";
import { errorText } from "./app";
import { t } from "../strings";

/** The one handle: what every endpoint's `execute` is given. */
export const handle: wire.client.Handle = wire.proxy.handle({
  invoke: (command, args) => invoke(command, args),
  channel: () => new Channel(),
});

/** The link's own commands: how it stands (dialling first), and dropping it so the next use dials again. */
export const link = {
  status: () => invoke<WireStatus>("wire_status"),
  reset: () => invoke<null>("wire_reset"),
};

/** A list endpoint as the hook needs it: how to open it, what tells its items apart, and what each frame means to the fold. */
export type ListSpec<T, F> = {
  open: (handle: wire.client.Handle) => Promise<[daemon.client.stream.ExecuteStream<F>, daemon.client.Cancel]>;
  key: shared.listing.Keyed<T>;
  change: (frame: F) => shared.listing.Change<T> | null;
};

/** A list as a screen holds it: the items so far, whether the daemon has said the list is whole, and why it ended, if it did. */
export type Held<T> = { items: T[]; listed: boolean; ended: string | null };

const EMPTY = { items: [], listed: false, ended: null };

/** Why a frame that is no change ended the list: forbidden, or the daemon's error, as text. */
function endedBy(frame: unknown): string {
  return frame === "forbidden" ? t.daemon.forbidden : JSON.stringify(frame);
}

/**
 * The list `spec` names, kept current while `enabled` and the
 * component lives: opened once, every frame folded, cancelled on the
 * way out. `spec` must be one object for the component's life (a
 * module constant, or memoized), since a new one reopens the list.
 */
export function useListing<T, F>(spec: ListSpec<T, F>, enabled: boolean): Held<T> {
  const [held, setHeld] = useState<Held<T>>(EMPTY);
  useEffect(() => {
    if (!enabled) {
      setHeld(EMPTY);
      return;
    }
    let stopped = false;
    let cancel: daemon.client.Cancel | null = null;
    const listing = new shared.listing.Listing<T>(spec.key);
    setHeld(EMPTY);
    (async () => {
      try {
        const [stream, opened] = await spec.open(handle);
        cancel = opened;
        for await (const frame of stream) {
          if (stopped) return;
          const change = spec.change(frame);
          if (change === null) {
            setHeld((h) => ({ ...h, ended: endedBy(frame) }));
            continue;
          }
          listing.push(change);
          setHeld((h) => ({ items: [...listing.items], listed: h.listed || change.kind === "listed", ended: null }));
        }
        if (!stopped) setHeld((h) => (h.ended ? h : { ...h, ended: t.daemon.ended }));
      } catch (error) {
        if (!stopped) setHeld((h) => ({ ...h, ended: errorText(error) }));
      }
    })();
    return () => {
      stopped = true;
      cancel?.cancel().catch(() => {});
    };
  }, [spec, enabled]);
  return held;
}

const agents = daemon.endpoints.agents.list;
export type Agent = daemon.endpoints.agents.list.server.response.Agent;
const AGENTS: ListSpec<Agent, daemon.endpoints.agents.list.server.response.Frame> = {
  open: (h) => agents.client.execute(h, {}),
  key: agents.server.response.key,
  change: agents.server.response.change,
};

/** Every agent the connection's grants reach, kept current. */
export function useAgents(enabled: boolean): Held<Agent> {
  return useListing(AGENTS, enabled);
}

const accounts = daemon.endpoints.accounts.list;
export type Account = daemon.endpoints.accounts.list.server.response.Account;
const ACCOUNTS: ListSpec<Account, daemon.endpoints.accounts.list.server.response.Frame> = {
  open: (h) => accounts.client.execute(h, {}),
  key: accounts.server.response.key,
  change: accounts.server.response.change,
};

/** Every account the connection's grants reach, kept current. */
export function useAccounts(enabled: boolean): Held<Account> {
  return useListing(ACCOUNTS, enabled);
}
