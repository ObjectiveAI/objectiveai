/**
 * The viewer's shared daemon executor — one lazy
 * [`ViewerCommandExecutor`]: commands travel over the Rust proxy's
 * `daemon_execute`, which owns the daemon address and stamps the
 * auth signature + the viewer agent identity server-side. Shared by
 * the viewer's one-off command reads (e.g. `useAgentDefinition`'s
 * definition fetch) and `AgentChat`'s message sends.
 */
import { ViewerCommandExecutor } from "@objectiveai/sdk";
import { viewerTransport } from "./viewer-transport";

let executorPromise: Promise<ViewerCommandExecutor> | null = null;

/** The shared executor, built once. Rejects outside Tauri. */
export async function daemonExecutor(): Promise<ViewerCommandExecutor> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const transport = await viewerTransport();
      if (!transport) {
        throw new Error(
          "daemon transport unavailable (not running under Tauri)",
        );
      }
      return new ViewerCommandExecutor(transport);
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
}
