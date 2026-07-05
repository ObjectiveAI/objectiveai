import { useEffect, useMemo, useRef, useState } from "react";
import { agentsInstancesListExecute } from "@objectiveai/sdk";
import { websocketExecutor } from "../lib/websocket-executor";
import { useActiveAgents } from "./useActiveAgents";

/** One known agent instance and whether it is active right now.
 * Identity-stable: the same object rides every returned list until
 * its `active` flag flips. */
export interface AgentStatus {
  agent_instance_hierarchy: string;
  active: boolean;
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
  const [listed, setListed] = useState<readonly string[] | null>(null);
  // Every AIH the stream has counted during this hook's lifetime —
  // ended agents stay known (marked inactive), in first-seen order.
  const seenRef = useRef<string[]>([]);
  // hier → its current AgentStatus object; replaced only on flips.
  const objectsRef = useRef(new Map<string, AgentStatus>());
  const previousRef = useRef<AgentStatus[]>([]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const executor = await websocketExecutor();
        const hierarchies: string[] = [];
        for await (const item of agentsInstancesListExecute(executor, {
          all: true,
          targets: [],
        })) {
          // In-band error lines are not instances.
          if ("type" in item) continue;
          hierarchies.push(item.agent_instance_hierarchy);
        }
        if (!cancelled) {
          setListed(hierarchies);
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
    // Accumulate stream-seen hierarchies (idempotent — safe under
    // StrictMode's double invocation).
    const seen = seenRef.current;
    const seenSet = new Set(seen);
    for (const agent of active) {
      if (!seenSet.has(agent.agent_instance_hierarchy)) {
        seenSet.add(agent.agent_instance_hierarchy);
        seen.push(agent.agent_instance_hierarchy);
      }
    }

    const activeSet = new Set(
      active.map((agent) => agent.agent_instance_hierarchy),
    );
    const objects = objectsRef.current;
    const next: AgentStatus[] = [];
    const included = new Set<string>();
    const push = (hier: string) => {
      if (included.has(hier)) return;
      included.add(hier);
      const isActive = activeSet.has(hier);
      let status = objects.get(hier);
      if (status === undefined || status.active !== isActive) {
        status = { agent_instance_hierarchy: hier, active: isActive };
        objects.set(hier, status);
      }
      next.push(status);
    };
    for (const hier of listed ?? []) push(hier);
    for (const hier of seen) push(hier);

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
