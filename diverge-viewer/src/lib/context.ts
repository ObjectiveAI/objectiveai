import { createContext, useContext } from "react";
import type { AppInfo } from "../bindings/AppInfo";
import type { TabKind } from "../bindings/TabKind";
import type { WireStatus } from "../bindings/WireStatus";
import type { Agent, Held } from "./daemon";

export type Shared = {
  /** The agents, as the daemon lists them, kept current. */
  agents: Held<Agent>;
  /** Whether the daemon is reached: nothing lists until it is. */
  connected: boolean;
  /** How the link stands, as Rust last said. */
  link: WireStatus;
  /** Drop the link and dial again. */
  redial: () => void;
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
