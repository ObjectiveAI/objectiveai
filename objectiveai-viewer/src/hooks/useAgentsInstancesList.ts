/**
 * The app's agents list, live from the daemon's
 * `/agents/instances/list` endpoint — a flat set of
 * `{ agent_instance_hierarchy, active }` items, snapshot on connect
 * then lock-driven activate/deactivate flips.
 *
 * NOT a singleton: the hook takes the daemon connection as an
 * argument (threaded down from App, which fetches it once) and owns
 * ONE `AgentsInstancesListListener` for its lifetime,
 * reconnecting with a 1s pause when the connection drops (each
 * reconnect re-snapshots, so the state self-heals). `null` connection
 * (browser dev, config not yet fetched) yields an empty list.
 */
import { useEffect, useState } from "react";
import {
  AgentsInstancesListListener,
  type CliAgentsInstancesListListenerAgentStatus,
} from "@objectiveai/sdk";
import type { DaemonConnection } from "../lib/daemon";
import { reportError } from "../lib/errors";

export type AgentStatus = CliAgentsInstancesListListenerAgentStatus;

export function useAgentsInstancesList(
  connection: DaemonConnection | null,
): AgentStatus[] {
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  useEffect(() => {
    if (connection === null) return;
    let cancelled = false;
    let current: AgentsInstancesListListener | null = null;
    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await AgentsInstancesListListener.connect(
            `${connection.address}/agents/instances/list`,
            {
              signature: connection.signature,
              onChange: (next) => {
                if (!cancelled) setAgents(next);
              },
            },
          );
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          setAgents(listener.agents());
          // Ride the connection until it closes (subscribe resolves on
          // every change AND on close).
          while (!listener.closed) {
            await listener.subscribe();
          }
        } catch (error) {
          // Connect refused / handshake failure — surface it, then retry.
          reportError("agents list", error);
        }
        current = null;
        if (cancelled) return;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    })();
    return () => {
      cancelled = true;
      current?.close();
    };
  }, [connection]);
  return agents;
}
