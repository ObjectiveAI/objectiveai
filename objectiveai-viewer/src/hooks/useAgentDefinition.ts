import { useEffect, useState } from "react";
import {
  agentsInstancesGetExecute,
  type CliCommandAgentsInstancesListResponseItem,
} from "@objectiveai/sdk";
import { logsTarget } from "../lib/aih";
import { daemonExecutor } from "../lib/executor";

/** The recorded agent definition: the remote path or the inline WF
 * spec, exactly as `agents instances get` returns it. */
export type AgentDefinition = NonNullable<
  CliCommandAgentsInstancesListResponseItem["agent"]
>;

/** [`useAgentDefinition`]'s result: the definition plus whether the
 * one-off read is still in flight. */
export interface AgentDefinitionResult {
  /** `null` while loading, when the read failed, or when nothing is
   * recorded for the hierarchy. */
  agent: AgentDefinition | null;
  loading: boolean;
}

/**
 * The recorded agent definition for one agent instance hierarchy —
 * a one-off `agents instances get` (its item's `agent` field, fed by
 * the CLI's agent_refs registry with the legacy request-blob
 * fallback). No live half: definitions only change when the agent
 * respawns by spec.
 */
export function useAgentDefinition(hierarchy: string): AgentDefinitionResult {
  const [agent, setAgent] = useState<AgentDefinition | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // A new hierarchy starts from scratch.
    setAgent(null);
    setLoading(true);
    let cancelled = false;

    void (async () => {
      try {
        const executor = await daemonExecutor();
        for await (const item of agentsInstancesGetExecute(executor, {
          targets: [logsTarget(hierarchy)],
        })) {
          // In-band error lines are not instances.
          if ("type" in item) continue;
          if (item.agent_instance_hierarchy !== hierarchy) continue;
          if (!cancelled) {
            setAgent(item.agent ?? null);
          }
        }
      } catch {
        // Daemon unreachable — nothing to show.
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [hierarchy]);

  return { agent, loading };
}
