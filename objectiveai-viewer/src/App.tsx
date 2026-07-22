import { useEffect, useRef, useState } from "react";
import cn from "classnames";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { viewerTransport } from "./lib/viewer-transport";
import type { ViewerTransport } from "@objectiveai/sdk";
import { isTauri, tauriListen } from "./lib/tauri";
import {
  tabsSnapshot,
  type TabDesc,
  type TabsSnapshot,
  type WindowTabs,
} from "./lib/tabs";
import {
  useAgentsInstancesList,
  type AgentStatus,
} from "./hooks/useAgentsInstancesList";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import { HierarchyTree } from "./components/HierarchyTree";
import { LaboratoriesPane } from "./components/LaboratoriesPane";
import { TabStrip } from "./components/TabStrip";
import { ConversationView } from "./components/ConversationView";
import { AgentChat } from "./components/AgentChat";
import { LaboratoryBrowser } from "./components/LaboratoryBrowser";
import { LogoMark, Wordmark } from "./components/shared/Logo";

/** The agents home tab: watermark + hierarchy tree. */
function AgentsPane({
  transport,
  agents,
  zoom,
}: {
  transport: ViewerTransport | null;
  agents: AgentStatus[];
  zoom: number;
}) {
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

/** One tab's content, by kind. Rendered ALWAYS (every tab in the
 * window stays mounted; visibility is CSS) so listeners and streams
 * keep running on background tabs. */
function TabContent({
  tab,
  transport,
  agents,
  zoom,
}: {
  tab: TabDesc;
  transport: ViewerTransport | null;
  agents: AgentStatus[];
  zoom: number;
}) {
  switch (tab.kind.type) {
    case "agents":
      return <AgentsPane transport={transport} agents={agents} zoom={zoom} />;
    case "laboratories":
      return <LaboratoriesPane transport={transport} />;
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

/** This window's label — the registry key for its slice of tabs. */
const WINDOW_LABEL = isTauri() ? getCurrentWebviewWindow().label : "main";

function App() {
  // This window's slice of the tab registry, rebuilt from every
  // `tabs://changed` snapshot (generation-guarded: a stale snapshot
  // response can never clobber a newer event).
  const [windowTabs, setWindowTabs] = useState<WindowTabs>({
    tabs: [],
    active: 0,
  });
  const generation = useRef(0);
  const [dockPreview, setDockPreview] = useState(false);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    let unlistenPreview: (() => void) | undefined;
    const apply = (snapshot: TabsSnapshot) => {
      if (disposed || snapshot.generation <= generation.current) return;
      generation.current = snapshot.generation;
      setWindowTabs(
        snapshot.windows[WINDOW_LABEL] ?? { tabs: [], active: 0 },
      );
    };
    void (async () => {
      // Subscribe FIRST (events are not queued for future listeners),
      // then snapshot; the generation guard orders the two.
      unlisten = await tauriListen<TabsSnapshot>("tabs://changed", (e) =>
        apply(e.payload),
      );
      unlistenPreview = await tauriListen<boolean>(
        "tabs://dock-preview",
        (e) => {
          if (!disposed) setDockPreview(e.payload);
        },
      );
      if (disposed) {
        unlisten?.();
        unlistenPreview?.();
        return;
      }
      const snapshot = await tabsSnapshot();
      if (snapshot) apply(snapshot);
    })();
    return () => {
      disposed = true;
      unlisten?.();
      unlistenPreview?.();
    };
  }, []);

  // The daemon transport (the Rust proxy's invoke + Channel), fetched
  // once per window. There is no global listener singleton — App
  // threads this down and components construct and own their own
  // listeners.
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
  // This window's agents-list connection: {aih, active} items, live.
  const agents = useAgentsInstancesList(transport);
  const activeAgents = agents.filter((agent) => agent.active).length;
  // Status-bar entries + canvas zoom (the footer slider drives it).
  const entries = useEntries();
  const [zoom, setZoom] = useState(1);

  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <TabStrip
        tabs={windowTabs.tabs}
        activeId={windowTabs.active}
        dockPreview={dockPreview}
      />
      <div className={cn("relative", "flex", "flex-col", "flex-1", "min-h-0")}>
        {windowTabs.tabs.length === 0 && (
          // A tab-less window (only possible on main — shells
          // auto-close empty): the brand mark plus a hint. Tabs can
          // be dragged back in.
          <div
            className={cn(
              "absolute",
              "inset-0",
              "flex",
              "flex-col",
              "items-center",
              "justify-center",
              "gap-3",
              "select-none",
            )}
          >
            <LogoMark className={cn("h-24", "w-auto", "text-info-dim/15")} />
            <Wordmark className={cn("w-[220px]", "h-auto", "text-info-dim/15")} />
            <div className={cn("font-mono", "text-[11px]", "text-info-dim")}>
              drag a tab here
            </div>
          </div>
        )}
        {/* Every tab stays mounted; only the active one is shown —
            background tabs keep their listeners and streams running. */}
        {windowTabs.tabs.map((tab) => (
          <div
            key={tab.id}
            className={cn(
              "flex-col",
              "flex-1",
              "min-h-0",
              tab.id === windowTabs.active ? "flex" : "hidden",
            )}
          >
            <TabContent
              tab={tab}
              transport={transport}
              agents={agents}
              zoom={zoom}
            />
          </div>
        ))}
      </div>
      <StatusBar
        entries={entries}
        activeAgents={activeAgents}
        zoom={zoom}
        onZoomChange={setZoom}
      />
    </div>
  );
}

export default App;
