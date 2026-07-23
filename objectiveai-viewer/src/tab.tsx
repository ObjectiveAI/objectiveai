/**
 * The CONTENT entry — one webview, one tab. The chrome webview
 * (`index.html`) renders the strip + footer; each tab's content runs
 * here, in its own child webview labeled `tab-<id>`, so pop-out /
 * pop-in is a native `reparent` and nothing here ever remounts.
 *
 * Identity comes from the webview LABEL (tab ids are minted by the
 * Rust registry and never reused); the tab's immutable kind is looked
 * up once from the registry snapshot. This entry deliberately does
 * NOT subscribe to `tabs://changed` — content doesn't care where its
 * tab lives, only what it is.
 */
import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import { isTauri } from "./lib/tauri";
import { viewerTransport } from "./lib/viewer-transport";
import { tabsSnapshot, uiGet, type TabDesc, type UiState } from "./lib/tabs";
import { setOrientation } from "./hooks/useOrientation";
import { TabContent } from "./components/TabContent";
import "./function-tree/styles/function-tree.css";
import "./app.css";

/** This webview's tab id, from its `tab-<id>` label. */
async function currentTabId(): Promise<number | null> {
  if (!isTauri()) return null;
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  const label = getCurrentWebview().label;
  if (!label.startsWith("tab-")) return null;
  const id = Number(label.slice("tab-".length));
  return Number.isInteger(id) && id > 0 ? id : null;
}

function TabRoot() {
  // The tab's descriptor, found once — kinds are immutable, so a
  // single snapshot lookup at boot is the whole registry dependency.
  const [tab, setTab] = useState<TabDesc | null>(null);
  useEffect(() => {
    let disposed = false;
    void (async () => {
      const tabId = await currentTabId();
      if (tabId === null) return;
      const snapshot = await tabsSnapshot();
      if (disposed || !snapshot) return;
      for (const windowTabs of Object.values(snapshot.windows)) {
        const found = windowTabs.tabs.find((t) => t.id === tabId);
        if (found) {
          setTab(found);
          return;
        }
      }
    })();
    return () => {
      disposed = true;
    };
  }, []);

  // This webview's own daemon transport (the Rust proxy routes each
  // Channel back to the webview that created it).
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

  // The hosting window's UI state (zoom / orientation), adopted live.
  // The listener MUST be webview-scoped: a plain `listen` has target
  // Any and would receive every other tab's targeted `ui://changed`
  // too. Listen first, then get — the boot-read race pattern.
  const [zoom, setZoom] = useState(1);
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const apply = (ui: UiState) => {
      if (disposed) return;
      setZoom(ui.zoom);
      setOrientation(ui.orientation);
    };
    void (async () => {
      if (!isTauri()) return;
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      unlisten = await getCurrentWebview().listen<UiState>(
        "ui://changed",
        (e) => apply(e.payload),
      );
      if (disposed) {
        unlisten?.();
        return;
      }
      const ui = await uiGet();
      if (ui) apply(ui);
    })();
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  if (!tab) return null;
  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <TabContent tab={tab} transport={transport} zoom={zoom} />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={300}>
      <TabRoot />
    </TooltipProvider>
  </React.StrictMode>,
);
