import { useEffect, useMemo, useRef, useState } from "react";
import { agentsInstancesListExecute } from "@objectiveai/sdk";
import { websocketExecutor } from "../lib/websocket-executor";
import { useActiveAgents } from "./useActiveAgents";

/** One known agent instance and whether it is active right now.
 * Identity-stable: the same object rides every returned list until
 * its `active` flag flips or its `created_at` resolves. */
export interface AgentStatus {
  agent_instance_hierarchy: string;
  active: boolean;
  /** Original spawn time (RFC3339). From the CLI instances list when
   * the agent was reported there (`null` when the CLI reported it
   * with no logs yet); for an agent NEVER reported by the CLI
   * command, locked in at the lowest last-active time encountered —
   * its `last_active_at` the moment the live tracker first saw it. */
  created_at: string | null;
}

/**
 * Every agent the viewer knows about — the union of a one-off
 * `agents instances list --all` (`agents/instances/list` with
 * `all: true`) and the live active-agents tracker — each marked
 * `active`. The live stream OVERRIDES the list: an agent the tracker
 * currently counts is `active: true` no matter what the list said,
 * and drops back to `active: false` (never removed) when its stream
 * ends. Stream-discovered agents missing from the list are appended
 * in first-seen order after the listed ones.
 *
 * The instances read runs once per mount; the live half re-renders on
 * every tracker change (shares `useActiveAgents`' global
 * registration, so nothing extra to register).
 */
export function useAgents(): AgentStatus[] {
  const active = useActiveAgents();
  const [listed, setListed] = useState<
    readonly { agent_instance_hierarchy: string; created_at: string | null }[] | null
  >(null);
  // Every AIH the stream has counted during this hook's lifetime →
  // its locked-in spawn time (the last_active_at at first sighting).
  // Ended agents stay known (marked inactive), in first-seen order.
  const seenRef = useRef(new Map<string, string>());
  // hier → its current AgentStatus object; replaced only on flips.
  const objectsRef = useRef(new Map<string, AgentStatus>());
  const previousRef = useRef<AgentStatus[]>([]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const executor = await websocketExecutor();
        const instances: {
          agent_instance_hierarchy: string;
          created_at: string | null;
        }[] = [];
        for await (const item of agentsInstancesListExecute(executor, {
          all: true,
          targets: [],
        })) {
          // In-band error lines are not instances.
          if ("type" in item) continue;
          instances.push({
            agent_instance_hierarchy: item.agent_instance_hierarchy,
            created_at: item.created_at ?? null,
          });
        }
        if (!cancelled) {
          setListed(instances);
        }
      } catch {
        // Daemon unreachable — the live tracker is the only source.
        if (!cancelled) {
          setListed([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return useMemo(() => {
    // Accumulate stream-seen hierarchies, locking each one's spawn
    // time at its first-seen last_active_at (idempotent — safe under
    // StrictMode's double invocation, and the lock never moves).
    const seen = seenRef.current;
    for (const agent of active) {
      if (!seen.has(agent.agent_instance_hierarchy)) {
        seen.set(agent.agent_instance_hierarchy, agent.last_active_at);
      }
    }

    const activeSet = new Set(
      active.map((agent) => agent.agent_instance_hierarchy),
    );
    // The CLI list is authoritative for spawn time when it reported
    // the agent (even as null); the lock only covers never-reported
    // agents.
    const listedCreatedAt = new Map(
      (listed ?? []).map((instance) => [
        instance.agent_instance_hierarchy,
        instance.created_at,
      ]),
    );
    const objects = objectsRef.current;
    const next: AgentStatus[] = [];
    const included = new Set<string>();
    const push = (hier: string) => {
      if (included.has(hier)) return;
      included.add(hier);
      const isActive = activeSet.has(hier);
      const createdAt = listedCreatedAt.has(hier)
        ? (listedCreatedAt.get(hier) ?? null)
        : (seen.get(hier) ?? null);
      let status = objects.get(hier);
      if (
        status === undefined ||
        status.active !== isActive ||
        status.created_at !== createdAt
      ) {
        status = {
          agent_instance_hierarchy: hier,
          active: isActive,
          created_at: createdAt,
        };
        objects.set(hier, status);
      }
      next.push(status);
    };
    for (const instance of listed ?? []) push(instance.agent_instance_hierarchy);
    for (const hier of seen.keys()) push(hier);

    // List identity: keep the previous array unless membership, order
    // or any member object changed.
    const previous = previousRef.current;
    if (
      previous.length === next.length &&
      previous.every((status, i) => status === next[i])
    ) {
      return previous;
    }
    previousRef.current = next;
    return next;
  }, [active, listed]);
}
