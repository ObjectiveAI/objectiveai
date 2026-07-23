/**
 * The CHROME entry's root — one per OS window, in the `chrome-<label>`
 * webview: the tab strip, the status bar, and the empty-window
 * watermark. Tab CONTENT lives in sibling `tab-<id>` webviews (the
 * `tab.html` entry) that the Rust shell places over the middle — the
 * chrome renders nothing there.
 *
 * The chrome owns the per-window UI controls (zoom / orientation) and
 * pushes them through `ui_set`; the Rust model fans them out to the
 * window's content webviews.
 */
import { useEffect, useRef, useState } from "react";
import cn from "classnames";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { viewerTransport } from "./lib/viewer-transport";
import type { ViewerTransport } from "@objectiveai/sdk";
import { isTauri, tauriListen } from "./lib/tauri";
import {
  seedHomeTabs,
  tabsSnapshot,
  uiSet,
  type TabsSnapshot,
  type WindowTabs,
} from "./lib/tabs";
import type { Orientation } from "./hooks/useOrientation";
import { useAgentsInstancesList } from "./hooks/useAgentsInstancesList";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import { TabStrip } from "./components/TabStrip";

/** This chrome's WINDOW label — the model key for its slice of tabs
 * (the chrome webview itself is labeled `chrome-<window>`; every
 * window is a shell-N, none is special). */
const WINDOW_LABEL = isTauri()
  ? getCurrentWebview().label.replace(/^chrome-/, "")
  : "shell-1";

function App() {
  // This window's slice of the shell model, rebuilt from every
  // `tabs://changed` snapshot (generation-guarded: a stale snapshot
  // response can never clobber a newer event).
  const [windowTabs, setWindowTabs] = useState<WindowTabs>({
    tabs: [],
    active: 0,
  });
  const generation = useRef(0);
  const [dockPreview, setDockPreview] = useState(false);
  // The one-shot home-tab seed: Rust boots an EMPTY window and knows
  // no tab names — when the whole model holds zero tabs (only ever
  // true for the boot chrome's first snapshot), THIS code opens the
  // home tabs through the same `tabs_open` API every identity uses.
  const seeded = useRef(false);

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
      if (
        !seeded.current &&
        Object.values(snapshot.windows).every((wt) => wt.tabs.length === 0)
      ) {
        seeded.current = true;
        void seedHomeTabs();
      }
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

  // The daemon transport (the Rust proxy's invoke + Channel) — the
  // chrome's OWN connection, for the footer's active-agents count
  // (content webviews hold their own; Channels route per webview).
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
  const agents = useAgentsInstancesList(transport);
  const activeAgents = agents.filter((agent) => agent.active).length;
  // Status-bar entries + the per-window UI controls. Zoom renders
  // locally (the slider) and pushes through `ui_set` for the content
  // webviews; orientation only pushes (the toggle's label reads the
  // local module store).
  const entries = useEntries();
  const [zoom, setZoom] = useState(1);
  const handleZoomChange = (next: number) => {
    setZoom(next);
    uiSet({ zoom: next });
  };
  const handleOrientationChange = (orientation: Orientation) => {
    uiSet({ orientation });
  };

  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <TabStrip
        tabs={windowTabs.tabs}
        activeId={windowTabs.active}
        dockPreview={dockPreview}
      />
      {/* The middle band belongs to the content webviews, composited
          above this document — the chrome paints bare ground beneath
          them. (A tab-less window doesn't exist: every window closes
          with its last tab, so there is no empty state.) */}
      <div className={cn("flex-1", "min-h-0")} />
      <StatusBar
        entries={entries}
        activeAgents={activeAgents}
        zoom={zoom}
        onZoomChange={handleZoomChange}
        onOrientationChange={handleOrientationChange}
      />
    </div>
  );
}

export default App;
