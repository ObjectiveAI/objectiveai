/**
 * The viewer's shared daemon `/execute` client — one lazy
 * [`SseCommandExecutor`] built from the Rust side's `websocket_config`,
 * which carries EVERY field the executor takes: the daemon address,
 * the auth signature (sent as the `X-OBJECTIVEAI-SIGNATURE` header),
 * and the agent arguments identifying viewer-initiated executions.
 * Shared by the plugin bridge's cli-execute forwarding and the
 * viewer's own one-off command reads (e.g. `useAgent`'s logs fetch).
 */
import { SseCommandExecutor } from "@objectiveai/sdk";
import { tauriInvoke } from "./tauri";

let executorPromise: Promise<SseCommandExecutor> | null = null;

/** The daemon config handed over by the Rust side, fetched once. */
export async function websocketExecutor(): Promise<SseCommandExecutor> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const config = await tauriInvoke<{
        address: string;
        signature: string | null;
        agent_arguments: Record<string, string | null>;
      }>("websocket_config");
      if (!config) {
        throw new Error(
          "websocket_config unavailable (not running under Tauri)",
        );
      }
      return new SseCommandExecutor(`${config.address}/execute`, {
        signature: config.signature,
        agentArguments: config.agent_arguments,
      });
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
}
