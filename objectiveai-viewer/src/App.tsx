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
import { LogoMark, Wordmark } from "./components/shared/Logo";

/** This chrome's WINDOW label — the model key for its slice of tabs
 * (the chrome webview itself is labeled `chrome-<window>`). */
const WINDOW_LABEL = isTauri()
  ? getCurrentWebview().label.replace(/^chrome-/, "")
  : "main";

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
          above this document — the chrome paints only the empty
          state beneath them. */}
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
      </div>
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
