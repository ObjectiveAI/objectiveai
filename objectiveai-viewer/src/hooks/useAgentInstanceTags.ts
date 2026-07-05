import { useEffect, useState } from "react";
import { agentsInstancesGetExecute } from "@objectiveai/sdk";
import { registerExecutionHandler } from "../daemon-listener";
import { logsTarget } from "../lib/aih";
import { websocketExecutor } from "../lib/websocket-executor";

/**
 * The live tag list for one agent instance hierarchy.
 *
 * A DYNAMIC listener registration (mount-scoped — the effect's
 * cleanup unregisters) watches every `agents/tags/apply` execution
 * and acts only when its unary response resolves WITHOUT error,
 * matching on the RESPONSE's resolved binding (requests can be
 * relative or tag-shaped):
 * - bound to this hierarchy → the tag name is added;
 * - bound anywhere else (a different AIH, or a non-instance
 *   grouped/agent binding) → the tag name is removed if held — a
 *   re-applied tag has left this agent. Unheld names no-op.
 *
 * State is then populated by a one-off `agents instances get` for
 * the hierarchy (its item's `tags`). The registration deliberately
 * comes FIRST and runs against the empty list until the read lands —
 * live-only, no buffering, no special-casing of that window.
 */
export function useAgentInstanceTags(hierarchy: string): string[] {
  const [tags, setTags] = useState<string[]>([]);

  useEffect(() => {
    // A new hierarchy starts from scratch.
    setTags([]);
    let cancelled = false;

    // 1. The dynamic registration — BEFORE the populate read.
    const unregister = registerExecutionHandler(
      "agents/tags/apply",
      (execution) => {
        void execution.response
          .then((response) => {
            if (cancelled) return;
            // In-band CliError resolution — not a successful apply.
            if ("type" in response) return;
            const bound =
              "agent_instance_hierarchy" in response
                ? response.agent_instance_hierarchy
                : null;
            const name = response.name;
            if (bound === hierarchy) {
              setTags((prev) =>
                prev.includes(name) ? prev : [...prev, name],
              );
            } else {
              // The tag bound elsewhere; if this agent held it, it
              // just left. Functional update keeps identity when
              // nothing changes.
              setTags((prev) =>
                prev.includes(name)
                  ? prev.filter((tag) => tag !== name)
                  : prev,
              );
            }
          })
          .catch(() => {
            // Settled without a response (socket closed) — not a
            // successful apply.
          });
      },
    );

    // 2. The one-off populate.
    void (async () => {
      try {
        const executor = await websocketExecutor();
        for await (const item of agentsInstancesGetExecute(executor, {
          targets: [logsTarget(hierarchy)],
        })) {
          // In-band error lines are not instances.
          if ("type" in item) continue;
          if (item.agent_instance_hierarchy !== hierarchy) continue;
          if (!cancelled) {
            setTags(item.tags);
          }
        }
      } catch {
        // Daemon unreachable — the registration keeps working over
        // whatever state exists.
      }
    })();

    return () => {
      cancelled = true;
      unregister();
    };
  }, [hierarchy]);

  return tags;
}
