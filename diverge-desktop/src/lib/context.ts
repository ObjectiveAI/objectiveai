import { createContext, useContext } from "react";
import type { AgentView } from "../bindings/AgentView";
import type { AppInfo } from "../bindings/AppInfo";
import type { CardView } from "../bindings/CardView";
import type { KnockView } from "../bindings/KnockView";
import type { SpaceSummary } from "../bindings/SpaceSummary";
import type { TabKind } from "../bindings/TabKind";

export type Shared = {
  agents: AgentView[];
  listedAt: number;
  refreshAgents: () => Promise<void>;
  info: AppInfo | null;
  spaces: SpaceSummary[];
  homeId: string | null;
  refreshSpaces: () => Promise<void>;
  knocks: KnockView[];
  cards: CardView[];
  /** Cards withdrawn because their agent stopped waiting: nothing answers them. */
  withdrawn: CardView[];
  answerCard: (id: number, answer: string) => Promise<void>;
  answerKnock: (knockId: number, yes: boolean) => Promise<void>;
  open: (tab: TabKind) => void;
  close: (key: string) => void;
};

export const SharedContext = createContext<Shared | null>(null);

export function useShared(): Shared {
  const value = useContext(SharedContext);
  if (!value) throw new Error("outside the app");
  return value;
}
