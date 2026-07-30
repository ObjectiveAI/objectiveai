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
import {
  ROOT_IDENTITY,
  identityAssetUrl,
  pluginAssetUrl,
  tabSelf,
  uiGet,
  type UiState,
} from "./lib/tabs";
import { setOrientation } from "./hooks/useOrientation";
import {
  TabHarnessProvider,
  type TabComponentProps,
} from "./lib/tabHarness";
import "./function-tree/styles/function-tree.css";
import "./app.css";

/** Inject one declared stylesheet and resolve when it has APPLIED.
 * A bundler strips `import "./x.css"` out of a JS entry and emits the
 * file beside it, so the tab's own module can never pull its styles
 * in — the shell does it here instead, from the descriptor.
 *
 * An href already in the document resolves immediately: a remounted
 * tab must not stack duplicate links. A load failure REJECTS, which
 * stops the tab rendering at all — an unstyled tab is a worse lie
 * than a missing one, and the build already refused any path that
 * didn't resolve, so reaching here means the file went missing after
 * install. */
function loadStylesheet(href: string): Promise<void> {
  return new Promise((resolve, reject) => {
    if (document.querySelector(`link[rel="stylesheet"][href="${CSS.escape(href)}"]`)) {
      resolve();
      return;
    }
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = href;
    link.addEventListener("load", () => resolve(), { once: true });
    link.addEventListener(
      "error",
      () => reject(new Error(`tab stylesheet failed to load: ${href}`)),
      { once: true },
    );
    document.head.append(link);
  });
}

/* Development hot-swap: replace every stylesheet link with a
 * cache-busted copy, in place. Insert-new-then-remove-old (no flash of
 * unstyled content), and deliberately NOT the boot failure path — a
 * failed swap keeps the old link and logs, where the boot path would
 * blank the tab. */
function swapStylesheets(version: number): void {
  const links = Array.from(
    document.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]'),
  );
  for (const link of links) {
    const base = link.href.split("?")[0];
    const fresh = document.createElement("link");
    fresh.rel = "stylesheet";
    fresh.href = `${base}?v=${version}`;
    const drop = () => link.remove();
    fresh.addEventListener("load", drop, { once: true });
    fresh.addEventListener(
      "error",
      () => {
        console.error(`dev: stylesheet swap failed: ${fresh.href}`);
        fresh.remove();
      },
      { once: true },
    );
    link.after(fresh);
  }
}

function TabRoot() {
  // The descriptor + the component it named, loaded once — kinds are
  // immutable, so this is the whole registry dependency. (Development
  // mode re-runs the load with a cache-busting token; see the
  // dev://module-changed listener below.)
  const [loaded, setLoaded] = useState<{
    Component: React.ComponentType<TabComponentProps>;
    arguments: unknown;
  } | null>(null);
  useEffect(() => {
    let disposed = false;
    /* The load body, callable: once at mount (bust = null), and again
     * per dev://module-changed with a version token. The token lands
     * in the module URL's query, which is what defeats the ES module
     * map — it memoizes per exact URL string, forever. */
    const load = async (bust: number | null) => {
      const descriptor = await tabSelf();
      if (!descriptor || disposed) return;
      // A plugin module lives under its own origin (the plugin://
      // protocol); root modules — and the root-template case flagged
      // by rootModule — resolve against the app origin as-is.
      const baseUrl =
        descriptor.identity === ROOT_IDENTITY || descriptor.rootModule
          ? descriptor.module
          : pluginAssetUrl(descriptor.identity, descriptor.module);
      const moduleUrl = bust === null ? baseUrl : `${baseUrl}?v=${bust}`;
      // The module and every declared stylesheet load CONCURRENTLY,
      // and the component renders only once all of them have — so
      // there is no flash of unstyled content, and the styles cost no
      // wall-clock beyond the module's own fetch.
      const [module] = await Promise.all([
        import(/* @vite-ignore */ moduleUrl) as Promise<
          Record<string, unknown>
        >,
        ...(descriptor.styles ?? []).map((style) =>
          loadStylesheet(identityAssetUrl(descriptor.identity, style)),
        ),
      ]);
      const component = module[descriptor.export ?? "default"];
      if (disposed || typeof component !== "function") return;
      setLoaded({
        Component: component as React.ComponentType<TabComponentProps>,
        arguments: descriptor.arguments,
      });
    };
    let unlistenModule: (() => void) | undefined;
    let unlistenStyles: (() => void) | undefined;
    void (async () => {
      // Development listeners FIRST (webview-scoped, like ui://changed
      // — a plain listen would receive every other tab's events too),
      // then the initial load: a change landing mid-boot re-runs the
      // load rather than being lost.
      if (isTauri()) {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        unlistenModule = await getCurrentWebview().listen<number>(
          "dev://module-changed",
          (e) => {
            // Component remount, document intact: the transport and
            // mailbox subscriptions survive; component state resets.
            void load(e.payload);
          },
        );
        unlistenStyles = await getCurrentWebview().listen<number>(
          "dev://styles-changed",
          (e) => swapStylesheets(e.payload),
        );
        if (disposed) {
          unlistenModule?.();
          unlistenStyles?.();
          return;
        }
      }
      await load(null);
    })();
    return () => {
      disposed = true;
      unlistenModule?.();
      unlistenStyles?.();
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
      // SEQUENTIAL on purpose — the listener must be attached before
      // the snapshot is taken, or a change landing in between is lost
      // (the same subscribe-then-snapshot ordering the chrome uses
      // for `tabs://changed`). Racing these would be a bug, not a
      // speedup.
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
