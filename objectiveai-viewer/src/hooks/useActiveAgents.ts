import { useEffect, useState } from "react";
import { registerExecutionHandler } from "../daemon-listener";

/** One currently-active agent instance. Identity-stable between
 * announcements: the object is replaced (with a fresh
 * `last_active_at`) each time the agent's AIH announcement arrives,
 * and keeps its reference otherwise — so consumers can key and memo
 * on it directly. */
export interface ActiveAgent {
  agent_instance_hierarchy: string;
  /** RFC3339; refreshed each time an AIH announcement arrives. */
  last_active_at: string;
}

/** The streaming execution paths that announce agent instance hierarchies. */
const TRACKED_PATHS = [
  "agents/spawn",
  "functions/execute/standard",
  "functions/execute/swiss_system",
] as const;

// ── The GLOBAL tracker ──────────────────────────────────────────────
// One refcounter for the whole app, shared by every hook caller: a
// single execution-handler registration on the daemon-listener
// singleton (register at viewer startup, before the listener starts)
// materializes the active list continuously — so a component mounting
// mid-execution reads the live snapshot instead of missing everything
// announced before it existed. React state stays per-caller: each
// mounted hook holds the current list in its own useState, synced by
// subscription.

/** AIH → refcount. Entries auto-delete at zero — the maps only ever
 * hold live agents. */
const counts = new Map<string, number>();
/** The live ActiveAgent object per hierarchy — replaced per
 * announcement (fresh `last_active_at`), dropped at zero, reused
 * across publishes in between. */
const objects = new Map<string, ActiveAgent>();
/** The currently-published list. Identity-stable: replaced only when
 * membership/order actually changes. */
let current: ActiveAgent[] = [];
const subscribers = new Set<(list: ActiveAgent[]) => void>();
let registered = false;

/** Publish the live list IFF membership/order changed; unchanged
 * members keep their exact objects. */
function publish(): void {
  const next = [...counts.keys()].map(
    (hier) => objects.get(hier) as ActiveAgent,
  );
  if (
    current.length === next.length &&
    current.every((v, i) => v === next[i])
  ) {
    return;
  }
  current = next;
  for (const notify of [...subscribers]) {
    notify(current);
  }
}

function increment(hier: string): void {
  counts.set(hier, (counts.get(hier) ?? 0) + 1);
  // Every announcement IS activity: a new object with a fresh
  // timestamp, so subscribers see the change.
  objects.set(hier, {
    agent_instance_hierarchy: hier,
    last_active_at: new Date().toISOString(),
  });
  publish();
}

function decrementAll(hiers: Set<string>): void {
  if (hiers.size === 0) return;
  for (const hier of hiers) {
    const count = (counts.get(hier) ?? 0) - 1;
    if (count > 0) {
      counts.set(hier, count);
    } else {
      // Auto-delete at zero — no dead entries accumulate.
      counts.delete(hier);
      objects.delete(hier);
    }
  }
  publish();
}

/**
 * Register the tracker's execution handler on the daemon-listener
 * singleton (idempotent, app-lifetime). Call at viewer startup,
 * BEFORE `startDaemonListener()` — the singleton is live-only, and a
 * late registration misses everything announced before it existed.
 */
export function registerActiveAgentsHandler(): void {
  if (registered) return;
  registered = true;
  registerExecutionHandler(TRACKED_PATHS, (execution) => {
    // Streaming forms only; the unary variants announce nothing.
    if (execution.request.dangerous_advanced?.stream !== true) return;
    const pathType = execution.request.path_type;
    // Count this execution's AIH announcements; release exactly them
    // when its stream ends (in-band errors don't end streams — the
    // terminator does).
    const announced = new Set<string>();
    return {
      onItem: (item) => {
        const hier = extractHierarchy(pathType, item);
        if (hier !== null && !announced.has(hier)) {
          announced.add(hier);
          increment(hier);
        }
      },
      onEnd: () => {
        decrementAll(announced);
      },
    };
  });
}

/**
 * Live list of active agent instance hierarchies, read from the
 * app-global refcounter (all callers share the counting; the returned
 * list is held in per-caller state). Tracks exactly the STREAMING
 * forms of `agents/spawn` and
 * `functions/execute/{standard,swiss_system}` —
 * `request.dangerous_advanced.stream === true`; each execution's AIH
 * announcements increment, and when the execution's stream ends its
 * hierarchies decrement, dropping at zero. First-seen order.
 *
 * AIH announcements on the wire:
 * - `agents/spawn`: the bare-string `Id` item IS the hierarchy.
 * - `functions/execute/*`: the tagged
 *   `{type: "agent_instance_hierarchy", agent_instance_hierarchy}`
 *   item (the bare-string item there is the execution id).
 *
 * Identity-preserving end to end: the array reference changes only
 * when membership or a member object changes (and is SHARED across
 * callers), and each agent keeps one object between announcements —
 * an announcement replaces it with a fresh `last_active_at`.
 */
export function useActiveAgents(): ActiveAgent[] {
  const [agents, setAgents] = useState<ActiveAgent[]>(() => current);

  useEffect(() => {
    // Catch anything published between render and subscription.
    setAgents(current);
    subscribers.add(setAgents);
    return () => {
      subscribers.delete(setAgents);
    };
  }, []);

  return agents;
}

/** Pull the AIH out of one response item, per the path's wire shape. */
function extractHierarchy(pathType: string, item: unknown): string | null {
  if (pathType === "agents/spawn") {
    return typeof item === "string" ? item : null;
  }
  // functions/execute/*: only the tagged announcement counts.
  if (
    item !== null &&
    typeof item === "object" &&
    (item as { type?: unknown }).type === "agent_instance_hierarchy" &&
    typeof (item as { agent_instance_hierarchy?: unknown })
      .agent_instance_hierarchy === "string"
  ) {
    return (item as { agent_instance_hierarchy: string })
      .agent_instance_hierarchy;
  }
  return null;
}
