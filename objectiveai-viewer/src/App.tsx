import { useState, useEffect } from "react";
import cn from "classnames";
import { viewerTransport } from "./lib/viewer-transport";
import type { ViewerTransport } from "@objectiveai/sdk";
import {
  useAgentsInstancesList,
  type AgentStatus,
} from "./hooks/useAgentsInstancesList";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import { ErrorToast } from "./components/ErrorToast";
import { HierarchyTree } from "./components/HierarchyTree";
import { LaboratoriesPane } from "./components/LaboratoriesPane";
import { TabBar, type Tab } from "./TabBar";
import { CommandPalette } from "./components/shared/CommandPalette";
import { LogoMark, Wordmark } from "./components/shared/Logo";
import type { Entry } from "./types";

/** The home pane's second-level tabs — the row below the main header
 * bar. `agents` is the historic hierarchy view; `laboratories` hosts
 * the laboratory builder. */
const HOME_TABS: Tab[] = [
  { id: "agents", label: "agents" },
  { id: "laboratories", label: "laboratories" },
];

function ObjectiveAIView({
  transport,
  agents,
  zoom,
  onStatusChange,
}: {
  transport: ViewerTransport | null;
  agents: AgentStatus[];
  zoom: number;
  onStatusChange?: (status: ViewerStatus) => void;
}) {
  const entries = useEntries();
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [homeTab, setHomeTab] = useState<string>("agents");

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setCommandPaletteOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  // Report the status-bar inputs up to App — the bar spans every tab,
  // so it lives above the panes.
  useEffect(() => {
    onStatusChange?.({ entries });
  }, [entries, onStatusChange]);

  return (
    <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
      <TabBar tabs={HOME_TABS} activeTab={homeTab} onSelect={setHomeTab} />
      {/* Both panes stay mounted; only the active one shows — the
          hierarchy tree's per-agent listeners keep running while the
          laboratories pane is focused. */}
      <div
        className={cn(
          "relative",
          "flex-1",
          "min-h-0",
          homeTab === "agents" ? "block" : "hidden",
        )}
      >
        <CommandPalette
          open={commandPaletteOpen}
          onOpenChange={setCommandPaletteOpen}
        />
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
      <div
        className={cn(
          "flex-col",
          "flex-1",
          "min-h-0",
          homeTab === "laboratories" ? "flex" : "hidden",
        )}
      >
        <LaboratoriesPane
          transport={transport}
          active={homeTab === "laboratories"}
        />
      </div>
    </div>
  );
}

/** The status-bar inputs ObjectiveAIView reports up to App. */
interface ViewerStatus {
  entries: Entry[];
}

function App() {
  const [status, setStatus] = useState<ViewerStatus>({
    entries: [],
  });
  // The daemon transport (the Rust proxy's invoke + Channel), fetched
  // once. There is no global listener singleton — App threads this
  // down and components construct and own their own listeners.
  const [transport, setTransport] = useState<ViewerTransport | null>(null);
  useEffect(() => {
    let cancelled = false;
    void viewerTransport().then((t) => {
      if (!cancelled && t !== null) setTransport(t);
    });
    return () => {
      cancelled = true;
    };
  }, []);
  // The app's ONE agents-list connection: {aih, active} items, live.
  const agents = useAgentsInstancesList(transport);
  const activeAgents = agents.filter((agent) => agent.active).length;
  // Canvas zoom — the footer slider drives it; the main canvas
  // consumes it.
  const [zoom, setZoom] = useState(1);

  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
        <ObjectiveAIView transport={transport} agents={agents} zoom={zoom} onStatusChange={setStatus} />
      </div>
      <StatusBar entries={status.entries} activeAgents={activeAgents} zoom={zoom} onZoomChange={setZoom} />
      <ErrorToast />
    </div>
  );
}

export default App;
