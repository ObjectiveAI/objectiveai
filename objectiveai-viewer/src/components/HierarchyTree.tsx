import cn from "classnames";
import { useAgents, type AgentStatus } from "../hooks/useAgents";

/** One agent scoped under a tree node: the path segments REMAINING
 * below that node, plus the agent itself. `rest` empty means the
 * node IS this agent. */
interface ScopedAgent {
  rest: readonly string[];
  status: AgentStatus;
}

/**
 * The whole-body agent hierarchy: every known agent's instance
 * hierarchy (from [`useAgents`]) split on `/` into a tree. Each node
 * shows ONLY its own segment — never a full hierarchy. A node can be
 * an agent, a branch, or both (an agent with child agents); branches
 * with no corresponding agent render dashed as pure structure.
 * Scrolls both ways.
 */
export function HierarchyTree() {
  const agents = useAgents();
  const roots = groupByHead(
    agents.map((status) => ({
      rest: status.agent_instance_hierarchy.split("/"),
      status,
    })),
  );
  return (
    <div className={cn("absolute", "inset-0", "overflow-auto")}>
      <div
        className={cn(
          "min-w-max",
          "p-6",
          "font-mono",
          "text-xs",
          "flex",
          "flex-col",
          "gap-1",
        )}
      >
        {roots.map(([name, members]) => (
          <HierarchyNode key={name} name={name} members={members} />
        ))}
      </div>
    </div>
  );
}

/**
 * One tree level. `members` is every agent under this node, each
 * carrying only its remaining path — the node finds itself (an agent
 * whose path ends here), groups the rest by their next segment, and
 * hands each child group down whole.
 */
function HierarchyNode({
  name,
  members,
}: {
  name: string;
  members: ScopedAgent[];
}) {
  // The agent AT this node, if any (hierarchies are unique — at most
  // one member terminates here).
  const self =
    members.find((member) => member.rest.length === 0)?.status ?? null;
  const children = groupByHead(
    members.filter((member) => member.rest.length > 0),
  );
  const kind =
    self === null ? "branch" : self.active ? "agent-active" : "agent-inactive";
  return (
    <div className={cn("flex", "flex-col", "gap-1")}>
      <div
        data-node-kind={kind}
        data-node-name={name}
        className={cn(
          "inline-flex",
          "items-center",
          "gap-1.5",
          "w-fit",
          "px-2",
          "py-0.5",
          "rounded-sm",
          "border",
          "whitespace-nowrap",
          "select-none",
          self === null
            ? // Pure structure: a path segment no agent occupies.
              cn("border-dashed", "border-node-border/70", "text-info-dim")
            : self.active
              ? cn(
                  "border-copper-dim",
                  "bg-copper-warm/10",
                  "text-copper-bright",
                )
              : cn("border-node-border", "bg-ground-surface", "text-info-mid"),
        )}
      >
        {self !== null && (
          <div
            className={cn(
              "w-1.5",
              "h-1.5",
              "rounded-full",
              "shrink-0",
              self.active ? cn("bg-copper-hot", "animate-pulse") : "bg-info-dim",
            )}
          />
        )}
        <span>{name}</span>
      </div>
      {children.length > 0 && (
        <div
          className={cn(
            "ml-2.5",
            "pl-3.5",
            "border-l",
            "border-node-border/60",
            "flex",
            "flex-col",
            "gap-1",
          )}
        >
          {children.map(([child, group]) => (
            <HierarchyNode key={child} name={child} members={group} />
          ))}
        </div>
      )}
    </div>
  );
}

/** Group agents by their next path segment, first-seen order, each
 * group's members re-scoped one level down. */
function groupByHead(scoped: ScopedAgent[]): [string, ScopedAgent[]][] {
  const groups = new Map<string, ScopedAgent[]>();
  for (const { rest, status } of scoped) {
    const head = rest[0];
    let group = groups.get(head);
    if (!group) {
      group = [];
      groups.set(head, group);
    }
    group.push({ rest: rest.slice(1), status });
  }
  return [...groups.entries()];
}
