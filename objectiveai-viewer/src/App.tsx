import { useState, useEffect } from "react";
import cn from "classnames";
import { tauriInvoke } from "./lib/tauri";
import { startDaemonListener } from "./daemon-listener";
import { registerActiveAgentsHandler } from "./hooks/useActiveAgents";
import { registerAgentCompletionsHandler } from "./listener-handlers/agentCompletions";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import { HierarchyTree } from "./components/HierarchyTree";
import { TabBar, type Tab } from "./TabBar";
import { PluginPane } from "./PluginPane";
import { CommandPalette } from "./components/shared/CommandPalette";
import { LogoMark, Wordmark } from "./components/shared/Logo";
import type { Entry } from "./types";

function ObjectiveAIView({ onStatusChange }: { onStatusChange?: (status: ViewerStatus) => void }) {
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
      <HierarchyTree />
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

  // Viewer startup: register every execution handler FIRST (the
  // listener is live-only), then start the autonomous singleton —
  // which registers the built-in plugins/run forwarding itself. All
  // idempotent, so StrictMode's double effect is harmless.
  useEffect(() => {
    registerActiveAgentsHandler();
    registerAgentCompletionsHandler();
    startDaemonListener();
  }, []);

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
          <ObjectiveAIView onStatusChange={setStatus} />
        </div>
        <StatusBar entries={status.entries} />
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
          <ObjectiveAIView onStatusChange={setStatus} />
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
      <StatusBar entries={status.entries} />
    </div>
  );
}

export default App;
