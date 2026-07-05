import { useEffect, useRef, useState } from "react";
import type { ListenerStream } from "./useListener";

/** The streaming run paths that announce agent instance hierarchies. */
const TRACKED_PATHS = new Set([
  "agents/spawn",
  "functions/execute/standard",
  "functions/execute/swiss_system",
]);

/**
 * Live list of active agent instance hierarchies (AIHs), refcounted
 * across the runs on `stream` (a [`ListenerStream`], normally from
 * `useListener`).
 *
 * Tracks exactly the STREAMING forms of `agents/spawn` and
 * `functions/execute/{standard,swiss_system}` —
 * `request.dangerous_advanced.stream === true`; everything else is
 * ignored. Each AIH announcement on a run's response increments that
 * hierarchy's count (once per run — the emitters announce each AIH
 * exactly once, and a per-run set guards it besides); when the run's
 * stream ends, every AIH it announced is decremented. The returned
 * list is the hierarchies with a live count, in first-seen order.
 *
 * AIH announcements on the wire:
 * - `agents/spawn`: the bare-string `Id` item IS the hierarchy.
 * - `functions/execute/*`: the tagged
 *   `{type: "agent_instance_hierarchy", agent_instance_hierarchy}`
 *   item (the bare-string item there is the execution id, NOT a
 *   hierarchy).
 *
 * Live-only like the feed itself: runs already in flight when the
 * stream subscribed are invisible.
 *
 * Identity-preserving, per the repo's merge-system philosophy: the
 * returned array keeps the SAME reference until its contents actually
 * change, and refcount moves that don't alter membership (a second
 * run holding an already-listed hierarchy) trigger no re-render at
 * all — the counts live in a ref; only the visible list is state.
 */
export function useActiveAgents(stream: ListenerStream): string[] {
  const countsRef = useRef<Map<string, number>>(new Map());
  const [agents, setAgents] = useState<string[]>(() => []);

  useEffect(() => {
    let alive = true;

    /** Publish the key list IFF membership/order changed. */
    const publish = () => {
      const keys = [...countsRef.current.keys()];
      setAgents((prev) =>
        prev.length === keys.length && prev.every((v, i) => v === keys[i])
          ? prev
          : keys,
      );
    };
    const increment = (hier: string) => {
      if (!alive) return;
      const counts = countsRef.current;
      counts.set(hier, (counts.get(hier) ?? 0) + 1);
      publish();
    };
    const decrementAll = (hiers: Set<string>) => {
      if (!alive || hiers.size === 0) return;
      const counts = countsRef.current;
      for (const hier of hiers) {
        const count = (counts.get(hier) ?? 0) - 1;
        if (count > 0) {
          counts.set(hier, count);
        } else {
          counts.delete(hier);
        }
      }
      publish();
    };

    /** Drain one tracked run: count its AIH announcements, release
     * them when its stream ends (in-band errors don't end streams —
     * the terminator does). */
    const track = (
      pathType: string,
      response: AsyncIterable<unknown>,
    ): void => {
      void (async () => {
        const announced = new Set<string>();
        try {
          for await (const item of response) {
            const hier = extractHierarchy(pathType, item);
            if (hier !== null && !announced.has(hier)) {
              announced.add(hier);
              increment(hier);
            }
          }
        } finally {
          decrementAll(announced);
        }
      })();
    };

    void (async () => {
      for await (const run of stream) {
        if (!alive) return;
        const request = run.request as {
          path_type: string;
          dangerous_advanced?: { stream?: boolean | null } | null;
        };
        if (!TRACKED_PATHS.has(request.path_type)) continue;
        if (request.dangerous_advanced?.stream !== true) continue;
        // Streaming form: the response is a response-item stream.
        // Subscribe synchronously on receipt — live-only.
        track(request.path_type, run.response as AsyncIterable<unknown>);
      }
    })();

    return () => {
      alive = false;
      countsRef.current = new Map();
      setAgents([]);
      // The outer loop ends when the stream's owner (useListener's
      // cleanup) return()s it; the per-run drains end with their runs.
    };
  }, [stream]);

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
