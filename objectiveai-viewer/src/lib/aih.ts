import type {
  CliCommandAgentsAgentSelector,
  CliCommandAgentsLogsListTarget,
} from "@objectiveai/sdk";

/** The direct command target for one AIH — the shape shared by the
 * `agents logs list` / `agents instances get` target unions: leaf
 * after the last `/`, lineage prefix as the parent (`null` for a
 * bare single-segment hierarchy). */
export function logsTarget(hierarchy: string): CliCommandAgentsLogsListTarget {
  const slash = hierarchy.lastIndexOf("/");
  if (slash === -1) {
    return {
      by: "direct",
      agent_instance: hierarchy,
      parent_agent_instance_hierarchy: null,
    };
  }
  return {
    by: "direct",
    agent_instance: hierarchy.slice(slash + 1),
    parent_agent_instance_hierarchy: hierarchy.slice(0, slash),
  };
}

/** The `agents message` / `agents spawn` instance selector for one
 * AIH — the same last-`/` split as [`logsTarget`], tagged
 * `by: "instance"` for the shared `AgentSelector` union. */
export function instanceSelector(
  hierarchy: string,
): CliCommandAgentsAgentSelector {
  const slash = hierarchy.lastIndexOf("/");
  if (slash === -1) {
    return {
      by: "instance",
      agent_instance: hierarchy,
      parent_agent_instance_hierarchy: null,
    };
  }
  return {
    by: "instance",
    agent_instance: hierarchy.slice(slash + 1),
    parent_agent_instance_hierarchy: hierarchy.slice(0, slash),
  };
}
