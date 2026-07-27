/**
 * The STRIP entry's root — one per OS window, in the `chrome-<label>`
 * webview, sized to the strip band alone.
 *
 * It is band-sized rather than full-window because a document that
 * spans the content band paints over it, and a BROWSER tab's surface
 * is a plain child window WebView2's compositor will cover regardless
 * of HWND z-order. The bottom bar is therefore its own webview (the
 * `status.html` entry) and the band between them belongs to the
 * `tab-<id>` content webviews alone.
 */
import { useEffect, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { isTauri, tauriListen } from "./lib/tauri";
import {
  declareChannelRequestTab,
  declareTabs,
  tabsSnapshot,
  type TabsSnapshot,
  type WindowTabs,
} from "./lib/tabs";
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

  // Declare the root tab inventory — the chrome's manifest-
  // equivalent. Every chrome declares on mount; Rust applies the
  // FIRST declaration per app run (later ones no-op), and the boot
  // orchestrator opens the enabled tabs.
  useEffect(() => {
    void declareTabs();
    void declareChannelRequestTab();
  }, []);


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

  // The strip IS this document — no wrapper, no spacer, no footer.
  // The webview is exactly the strip band, so anything else here would
  // simply be clipped.
  return (
    <TabStrip
      tabs={windowTabs.tabs}
      activeId={windowTabs.active}
      dockPreview={dockPreview}
    />
  );
}

export default App;
