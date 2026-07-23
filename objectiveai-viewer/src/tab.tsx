/**
 * The CONTENT bootstrap — one webview, one tab, ZERO routing
 * knowledge. The chrome webview (`index.html`) renders the strip +
 * footer; each tab's content runs here, in its own child webview
 * labeled `tab-<id>`, so pop-out / pop-in is a native `reparent` and
 * nothing here ever remounts.
 *
 * This document is a dumb executor of whatever Rust says: it asks
 * `tab_self` for its descriptor, dynamic-imports the module Rust
 * chose, and renders the named export under the harness. There is no
 * switch, no name table, no resolver — a built-in tab and (later) a
 * plugin tab differ only in the descriptor and the origin serving
 * them. It deliberately does NOT subscribe to `tabs://changed` —
 * content doesn't care where its tab lives, only what it is.
 */
import React, { useEffect, useState } from "react";
import ReactDOM from "react-dom/client";
import { Provider as TooltipProvider } from "@radix-ui/react-tooltip";
import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import { isTauri } from "./lib/tauri";
import { viewerTransport } from "./lib/viewer-transport";
import { tabSelf, uiGet, type UiState } from "./lib/tabs";
import { setOrientation } from "./hooks/useOrientation";
import {
  TabHarnessProvider,
  type TabComponentProps,
} from "./lib/tabHarness";
import "./function-tree/styles/function-tree.css";
import "./app.css";

function TabRoot() {
  // The descriptor + the component it named, loaded once — kinds are
  // immutable, so this is the whole registry dependency.
  const [loaded, setLoaded] = useState<{
    Component: React.ComponentType<TabComponentProps>;
    arguments: unknown;
  } | null>(null);
  useEffect(() => {
    let disposed = false;
    void (async () => {
      const descriptor = await tabSelf();
      if (!descriptor || disposed) return;
      const module = (await import(
        /* @vite-ignore */ descriptor.module
      )) as Record<string, unknown>;
      const component = module[descriptor.export ?? "default"];
      if (disposed || typeof component !== "function") return;
      setLoaded({
        Component: component as React.ComponentType<TabComponentProps>,
        arguments: descriptor.arguments,
      });
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

  if (loaded === null) return null;
  const { Component } = loaded;
  return (
    <div className={cn("flex", "flex-col", "h-screen")}>
      <TabHarnessProvider value={{ transport, zoom }}>
        <Component arguments={loaded.arguments} />
      </TabHarnessProvider>
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
