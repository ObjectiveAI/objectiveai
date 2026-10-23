import { createContext, useContext } from "react";
import type { AppInfo } from "../bindings/AppInfo";
import type { TabKind } from "../bindings/TabKind";

/** One agent as the rail and the tabs name it: the daemon's own key, and the name beside. */
export type AgentRef = { template: string; index: number; name: string | null; active: boolean; lastActive: string | null };

export type Shared = {
  /** The agents, as the daemon lists them, kept current. */
  agents: AgentRef[];
  /** Whether the daemon is reached: nothing lists until it is. */
  connected: boolean;
  info: AppInfo | null;
  open: (tab: TabKind) => void;
  close: (key: string) => void;
};

export const SharedContext = createContext<Shared | null>(null);

export function useShared(): Shared {
  const value = useContext(SharedContext);
  if (!value) throw new Error("outside the app");
  return value;
}

/** The tab key of an agent: the daemon's key, as Rust spells it. */
export function agentKey(a: { template: string; index: number }): string {
  return `agent:${a.template}:${a.index}`;
}
