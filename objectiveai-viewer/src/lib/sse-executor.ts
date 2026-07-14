/**
 * The viewer's shared daemon `/execute` client — one lazy
 * [`SseCommandExecutor`] built from the Rust side's `daemon_config`,
 * which carries EVERY field the executor takes: the daemon address,
 * the auth signature (sent as the `X-OBJECTIVEAI-SIGNATURE` header),
 * and the agent arguments identifying viewer-initiated executions.
 * Shared by the plugin bridge's cli-execute forwarding and the
 * viewer's own one-off command reads (e.g. `useAgent`'s logs fetch).
 */
import { SseCommandExecutor, type CliCommandAgentArguments } from "@objectiveai/sdk";
import { tauriInvoke } from "./tauri";

let executorPromise: Promise<SseCommandExecutor> | null = null;

/** The daemon config handed over by the Rust side, fetched once. */
export async function sseExecutor(): Promise<SseCommandExecutor> {
  if (!executorPromise) {
    executorPromise = (async () => {
      const config = await tauriInvoke<{
        address: string;
        signature: string | null;
        agent_arguments: CliCommandAgentArguments;
      }>("daemon_config");
      if (!config) {
        throw new Error(
          "daemon_config unavailable (not running under Tauri)",
        );
      }
      return new SseCommandExecutor(`${config.address}/execute`, {
        signature: config.signature,
        agentInstanceHierarchy: config.agent_arguments.agent_instance_hierarchy,
        agentId: config.agent_arguments.agent_id,
        agentFullId: config.agent_arguments.agent_full_id,
        agentRemote: config.agent_arguments.agent_remote,
        responseId: config.agent_arguments.response_id,
        responseIds: config.agent_arguments.response_ids,
      });
    })();
    // A failed fetch shouldn't poison every later invocation.
    executorPromise.catch(() => {
      executorPromise = null;
    });
  }
  return executorPromise;
}
