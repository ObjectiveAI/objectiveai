/**
 * The STATUS entry — one webview per OS window, `status-<label>`,
 * holding the bottom bar and nothing else.
 *
 * It is a SEPARATE webview from the tab strip (`index.html`) for one
 * reason: between them lies the content band, and whatever document
 * spans that band paints over it. For a `tab-<id>` content webview
 * that is invisible — two WebView2 surfaces composited in the same
 * DirectComposition tree, ordered as expected. For a BROWSER tab it is
 * fatal: CEF paints a plain child window, which WebView2's compositor
 * covers regardless of HWND z-order and regardless of WS_CLIPSIBLINGS.
 * One full-window chrome document could not be made to stop doing
 * that; two band-sized ones never start.
 *
 * The split costs this window a second daemon transport (for the
 * active-agents count) and a second entries subscription. That is the
 * price of the content band belonging to the content alone.
 */
import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import type { ViewerTransport } from "@objectiveai/sdk";
import { viewerTransport } from "./lib/viewer-transport";
import { uiSet } from "./lib/tabs";
import type { Orientation } from "./hooks/useOrientation";
import { useAgentsInstancesList } from "./hooks/useAgentsInstancesList";
import { useEntries } from "./hooks/useEntries";
import { StatusBar } from "./components/layout/StatusBar";
import "./app.css";

function StatusRoot() {
  // This webview's OWN daemon connection. Channels route per webview,
  // so the strip's cannot be shared with it.
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
  const entries = useEntries();

  // Zoom renders locally (the slider) and pushes through `ui_set` for
  // the window's content webviews; orientation only pushes (the
  // toggle's label reads the local module store).
  const [zoom, setZoom] = useState(1);
  const handleZoomChange = (next: number) => {
    setZoom(next);
    uiSet({ zoom: next });
  };
  const handleOrientationChange = (orientation: Orientation) => {
    uiSet({ orientation });
  };

  return (
    <StatusBar
      entries={entries}
      activeAgents={activeAgents}
      zoom={zoom}
      onZoomChange={handleZoomChange}
      onOrientationChange={handleOrientationChange}
    />
  );
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <TooltipProvider delayDuration={300}>
      <StatusRoot />
    </TooltipProvider>
  </React.StrictMode>,
);
