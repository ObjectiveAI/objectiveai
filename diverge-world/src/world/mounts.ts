// Which containers a volume is mounted into: the keys of the agents and
// tools that name it in their mounts, as the board draws them.

import type { daemon } from "@diverge/sdk";
import type { Snapshot } from "./snapshot";

type Volume = daemon.endpoints.volumes.list.server.response.Volume;
type Owner = daemon.key.Owner;
type Listed = daemon.endpoints.dependency_tools.list.server.response.Listed;

/** An owner as one string, an agent's name left off: how a dependency tool's owner is matched across lists. */
function ownerId(owner: Owner | Listed["owner"]): string {
  if (owner === "global") return "global";
  if ("agent" in owner) return `agent:${owner.agent.template}:${owner.agent.index}`;
  return `template:${owner.agent_template.template}`;
}

/**
 * The containers of a volume, by the keys the board draws them under:
 * every agent and standalone tool that names it in its mounts, and every
 * dependency tool among those the snapshot listed. A connected tool runs
 * on another daemon and mounts nothing here, and is left out.
 */
export function mountsOf(snapshot: Snapshot): (volume: Volume) => string[] {
  const dependencies = new Map<string, string>();
  for (const d of snapshot.dependencies) dependencies.set(`${d.id}\n${ownerId(d.owner)}`, `dependency_tool:${d.id}:${d.index}`);
  return (volume) => {
    const keys = (volume.agents ?? []).map((a) => `agent:${a.template}:${a.index}`);
    for (const tool of volume.tools ?? []) {
      if ("record" in tool) keys.push(`standalone_tool:${tool.record.template}:${tool.record.index}`);
      else if ("dependency" in tool) {
        const key = dependencies.get(`${tool.dependency.template}\n${ownerId(tool.dependency.owner)}`);
        if (key) keys.push(key);
      }
    }
    return keys;
  };
}
