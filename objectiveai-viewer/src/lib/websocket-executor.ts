/**
 * The viewer's shared daemon `/execute` client — one lazy
 * [`WebSocketExecutor`] built from the Rust side's `daemon_config`,
 * identifying every command as the Viewer. Shared by the plugin
 * bridge's cli-execute forwarding and the viewer's own one-off
 * command reads (e.g. `useAgent`'s logs fetch).
 */
import { WebSocketExecutor } from "@objectiveai/sdk";
import { tauriInvoke } from "./tauri";

let executorPromise: Promise<WebSocketExecutor> | null = null;

/** The daemon config handed over by the Rust side, fetched once. */
export async function websocketExecutor(): Promise<WebSocketExecutor> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const config = await tauriInvoke<{
        address: string;
        signature: string | null;
      }>("daemon_config");
      if (!config) {
        throw new Error("daemon_config unavailable (not running under Tauri)");
      }
      return new WebSocketExecutor(`${config.address}/execute`, {
        signature: config.signature,
        agentArguments: { agent_instance_hierarchy: "Viewer" },
      });
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
}
