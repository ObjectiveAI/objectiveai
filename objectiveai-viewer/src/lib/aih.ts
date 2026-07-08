import type { CliCommandAgentsLogsListTarget } from "@objectiveai/sdk";

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
