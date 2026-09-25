import { createContext, useContext } from "react";
import type { AgentView } from "../bindings/AgentView";
import type { AppInfo } from "../bindings/AppInfo";
import type { TabKind } from "../bindings/TabKind";

export type Shared = {
  agents: AgentView[];
  listedAt: number;
  refreshAgents: () => Promise<void>;
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
