import { useState, useEffect } from "react";
import cn from "classnames";
import { tauriInvoke } from "./lib/tauri";
import { daemonConnection, type DaemonConnection } from "./lib/daemon";
import {
  useAgentsInstancesList,
  type AgentStatus,
} from "./hooks/useAgentsInstancesList";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import { ErrorToast } from "./components/ErrorToast";
import { HierarchyTree } from "./components/HierarchyTree";
import { TabBar, type Tab } from "./TabBar";
import { PluginPane } from "./PluginPane";
import { CommandPalette } from "./components/shared/CommandPalette";
import { LogoMark, Wordmark } from "./components/shared/Logo";
import type { Entry } from "./types";

function ObjectiveAIView({
  connection,
  agents,
  onStatusChange,
}: {
  connection: DaemonConnection | null;
  agents: AgentStatus[];
  onStatusChange?: (status: ViewerStatus) => void;
}) {
  const entries = useEntries();
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);

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
    <div className={cn("relative", "flex-1", "min-h-0")}>
      <CommandPalette open={commandPaletteOpen} onOpenChange={setCommandPaletteOpen} />
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
      <HierarchyTree connection={connection} agents={agents} />
    </div>
  );
}

const OBJECTIVEAI_TAB_ID = "objectiveai";

export interface ViewerPluginInfo {
  owner: string;
  name: string;
  version: string;
  iframe_src: string;
}

/** The status-bar inputs ObjectiveAIView reports up to App. */
interface ViewerStatus {
  entries: Entry[];
}

function App() {
  const [plugins, setPlugins] = useState<ViewerPluginInfo[]>([]);
  const [activeTab, setActiveTab] = useState<string>(OBJECTIVEAI_TAB_ID);
  const [status, setStatus] = useState<ViewerStatus>({
    entries: [],
  });
  // The daemon connection coordinates, fetched once. There is no
  // global listener singleton — App threads this down and components
  // construct and own their own listeners.
  const [connection, setConnection] = useState<DaemonConnection | null>(null);
  useEffect(() => {
    let cancelled = false;
    void daemonConnection().then((config) => {
      if (!cancelled && config !== null) setConnection(config);
    });
    return () => {
      cancelled = true;
    };
  }, []);
  // The app's ONE agents-list connection: {aih, active} items, live.
  const agents = useAgentsInstancesList(connection);
  const activeAgents = agents.filter((agent) => agent.active).length;

  useEffect(() => {
    tauriInvoke<ViewerPluginInfo[]>("list_plugins_with_viewer")
      .then((p) => { if (p) setPlugins(p); })
      .catch((e) => {
        // eslint-disable-next-line no-console
        console.warn("list_plugins_with_viewer failed:", e);
      });
  }, []);

  const tabs: Tab[] = [
    { id: OBJECTIVEAI_TAB_ID, label: "ObjectiveAI" },
    ...plugins.map((p) => ({ id: p.name, label: p.name })),
  ];

  if (plugins.length === 0) {
    return (
      <div className={cn("flex", "flex-col", "h-screen")}>
        <div className={cn("flex", "flex-col", "flex-1", "min-h-0")}>
          <ObjectiveAIView connection={connection} agents={agents} onStatusChange={setStatus} />
        </div>
        <StatusBar entries={status.entries} activeAgents={activeAgents} />
      <ErrorToast />
      </div>
    );
  }

  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <div
        className={cn(
          "flex",
          "flex-row",
          "items-stretch",
          "bg-neutral-100",
          "dark:bg-neutral-900",
          "border-b",
          "border-neutral-300",
          "dark:border-neutral-700",
        )}
      >
        <div className={cn("flex-1", "min-w-0")}>
          <TabBar tabs={tabs} activeTab={activeTab} onSelect={setActiveTab} />
        </div>
      </div>
      <div
        className={cn(
          "relative",
          "flex",
          "flex-col",
          "flex-1",
          "min-h-0",
        )}
      >
        {/* Every pane stays mounted at all times; only the active one is
            shown (the rest are display:none). Keeping plugin iframes
            mounted means their JS keeps running and the bridge keeps
            their per-plugin Tauri subscription alive, so a plugin
            receives its routed daemon-stream events (`plugins_run`)
            regardless of which tab is focused. */}
        <div
          className={cn(
            "flex-col",
            "flex-1",
            "min-h-0",
            activeTab === OBJECTIVEAI_TAB_ID ? "flex" : "hidden",
          )}
        >
          <ObjectiveAIView connection={connection} agents={agents} onStatusChange={setStatus} />
        </div>
        {plugins.map((p) => (
          <div
            key={p.name}
            className={cn(
              "flex-col",
              "flex-1",
              "min-h-0",
              activeTab === p.name ? "flex" : "hidden",
            )}
          >
            <PluginPane info={p} />
          </div>
        ))}
      </div>
      {/* Spans every tab — plugin panes included. */}
      <StatusBar entries={status.entries} activeAgents={activeAgents} />
      <ErrorToast />
    </div>
  );
}

export default App;
