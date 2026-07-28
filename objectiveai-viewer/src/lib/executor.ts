/**
 * The viewer's shared daemon client — one lazy viewer-mode
 * [`Client`]: the client IS the executor, and commands travel over
 * the Rust proxy's `daemon_execute`, which owns the daemon address
 * and stamps the auth signature + the viewer agent identity
 * server-side. Shared by the viewer's one-off command reads (e.g.
 * `useAgentDefinition`'s definition fetch), `AgentChat`'s message
 * sends, and every hook's listener mint.
 */
import { Client } from "@objectiveai/sdk";
import { viewerTransport } from "./viewer-transport";

let executorPromise: Promise<Client> | null = null;

/** The shared viewer-mode client, built once. Rejects outside
 * Tauri. */
export async function daemonExecutor(): Promise<Client> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const transport = await viewerTransport();
      if (!transport) {
        throw new Error(
          "daemon transport unavailable (not running under Tauri)",
        );
      }
      return Client.viewer(transport);
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
}
