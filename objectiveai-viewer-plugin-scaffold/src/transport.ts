/**
 * The viewer transport, built from Tauri's IPC — the ONE injection
 * point the SDK's viewer helpers take (`openViewerTab`,
 * `subscribeViewerTab`, `Client.viewer`, …). The SDK never imports
 * Tauri itself; a plugin tab builds this once and threads it.
 */
import type { ViewerTransport } from "@objectiveai/sdk";

let cached: Promise<ViewerTransport> | null = null;

export function transport(): Promise<ViewerTransport> {
  if (!cached) {
    cached = (async () => {
      const { invoke, Channel } = await import("@tauri-apps/api/core");
      const built: ViewerTransport = {
        invoke: (cmd, args) => invoke(cmd, args),
        channel: <T,>() => new Channel<T>(),
      };
      return built;
    })();
  }
  return cached;
}
