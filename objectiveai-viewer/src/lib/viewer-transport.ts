/**
 * The daemon transport: Tauri `invoke` + `Channel`, structurally
 * typed for the SDK's viewer-mode constructors (`connectViewer` /
 * `ViewerCommandExecutor`). Every daemon stream rides the Rust
 * side's `daemon_*` proxy commands over IPC — the webview holds no
 * daemon connections, address, or credentials. `null` outside Tauri
 * (browser dev) — no daemon is reachable there.
 *
 * Cached; the dynamic import keeps `@tauri-apps/api` out of
 * plain-browser bundles' hot path (`withGlobalTauri` stays false).
 * App fetches this once and threads it (and/or data derived from it)
 * down as props; components construct and own their own listeners —
 * there is deliberately NO global listener singleton.
 */
import type { ViewerTransport } from "@objectiveai/sdk";
import { isTauri } from "./tauri";

let transportPromise: Promise<ViewerTransport | null> | null = null;

export function viewerTransport(): Promise<ViewerTransport | null> {
  if (!transportPromise) {
    transportPromise = (async () => {
      if (!isTauri()) return null;
      const { invoke, Channel } = await import("@tauri-apps/api/core");
      const transport: ViewerTransport = {
        invoke: (cmd, args) => invoke(cmd, args),
        channel: <T,>() => new Channel<T>(),
      };
      return transport;
    })();
  }
  return transportPromise;
}
