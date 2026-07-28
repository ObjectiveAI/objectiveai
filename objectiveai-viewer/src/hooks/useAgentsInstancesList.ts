/**
 * The app's agents list, live from the daemon's
 * `/agents/instances/list` endpoint — a flat set of
 * `{ agent_instance_hierarchy, active }` items, snapshot on connect
 * then lock-driven activate/deactivate flips.
 *
 * NOT a singleton: the hook takes the daemon transport as an
 * argument (threaded down from App, which fetches it once) and owns
 * ONE `AgentsInstancesListListener` for its lifetime,
 * reconnecting with a 1s pause when the connection drops (each
 * reconnect re-snapshots, so the state self-heals). `null` transport
 * (browser dev, not yet fetched) yields an empty list.
 */
import { useEffect, useState } from "react";
import {
  AgentsInstancesListListener,
  Client,
  type DaemonAgentsInstancesListListenerAgentStatus,
  type ViewerTransport,
} from "@objectiveai/sdk";
import { reportError } from "../lib/errors";

export type AgentStatus = DaemonAgentsInstancesListListenerAgentStatus;

export function useAgentsInstancesList(
  transport: ViewerTransport | null,
): AgentStatus[] {
  const [agents, setAgents] = useState<AgentStatus[]>([]);
  useEffect(() => {
    if (transport === null) return;
    let cancelled = false;
    let current: AgentsInstancesListListener | null = null;
    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await Client.viewer(
            transport,
          ).agentsInstancesListListener();
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          setAgents(listener.agents());
          // Ride the connection until it closes (subscribe resolves on
          // every change AND on close), pushing the fold each wake.
          while (!listener.closed) {
            await listener.subscribe();
            if (cancelled) return;
            setAgents(listener.agents());
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
  }, [transport]);
  return agents;
}
