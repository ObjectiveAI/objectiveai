import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import type { TabDesc } from "../lib/tabs";
import { useAgentsInstancesList } from "../hooks/useAgentsInstancesList";
import { HierarchyTree } from "./HierarchyTree";
import { LaboratoriesPane } from "./LaboratoriesPane";
import { ViewerLogsPane } from "./ViewerLogsPane";
import { ConversationView } from "./ConversationView";
import { AgentChat } from "./AgentChat";
import { LaboratoryBrowser } from "./LaboratoryBrowser";
import { LogoMark, Wordmark } from "./shared/Logo";

/** The agents home tab: watermark + hierarchy tree. Owns its own
 * agents-list connection — content is self-contained (it may render
 * in its own webview, away from the chrome's footer count). */
function AgentsPane({
  transport,
  zoom,
}: {
  transport: ViewerTransport | null;
  zoom: number;
}) {
  const agents = useAgentsInstancesList(transport);
  return (
    <div className={cn("relative", "flex-1", "min-h-0")}>
      {/* The brand mark: perfectly centered, always behind the body. */}
      <div
        className={cn(
          "absolute",
          "inset-0",
          "flex",
          "flex-col",
          "items-center",
          "justify-center",
          "gap-3",
          "pointer-events-none",
          "select-none",
        )}
      >
        <LogoMark className={cn("h-24", "w-auto", "text-info-dim/15")} />
        <Wordmark className={cn("w-[220px]", "h-auto", "text-info-dim/15")} />
      </div>
      {/* The body: the agent hierarchy tree, over the watermark. */}
      <HierarchyTree transport={transport} agents={agents} zoom={zoom} />
    </div>
  );
}

/** One tab's content, by kind — self-contained: everything it needs
 * beyond the transport and the chrome-driven zoom comes from its own
 * hooks/connections. */
export function TabContent({
  tab,
  transport,
  zoom,
}: {
  tab: TabDesc;
  transport: ViewerTransport | null;
  zoom: number;
}) {
  switch (tab.kind.type) {
    case "agents":
      return <AgentsPane transport={transport} zoom={zoom} />;
    case "laboratories":
      return <LaboratoriesPane transport={transport} />;
    case "viewer_logs":
      return <ViewerLogsPane />;
    case "agent":
      return (
        <>
          <div className={cn("flex-1", "min-h-0")}>
            <ConversationView transport={transport} hierarchy={tab.kind.aih} />
          </div>
          <AgentChat hierarchy={tab.kind.aih} />
        </>
      );
    case "laboratory":
      return (
        <LaboratoryBrowser
          transport={transport}
          id={tab.kind.id}
          machine={tab.kind.machine ?? undefined}
          machineState={tab.kind.machine_state ?? undefined}
        />
      );
  }
}
