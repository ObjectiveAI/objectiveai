import cn from "classnames";
import { useAgents, type AgentStatus } from "../hooks/useAgents";
import { useAgentInstanceTags } from "../hooks/useAgentInstanceTags";

/** One agent scoped under a tree node: the path segments REMAINING
 * below that node, plus the agent itself. `rest` empty means the
 * node IS this agent. */
interface ScopedAgent {
  rest: readonly string[];
  status: AgentStatus;
}

/**
 * The whole-body agent hierarchy: every known agent's instance
 * hierarchy (from [`useAgents`]) split on `/` into an org-chart
 * tree — TIERS stack top-down, siblings flow left-to-right on the
 * same level (`fizz/foo/bar` + `fizz/foo/buzz` → three tiers, with
 * `bar` and `buzz` side by side under `foo`). Everything anchors
 * LEFT: a parent sits at its subtree's leading edge, with the first
 * child directly below it. Each node shows ONLY
 * its own segment — never a full hierarchy. Agents render as boxes
 * carrying their spawn and last-active times (locale-formatted);
 * branches with no corresponding agent render dashed as pure
 * structure. Scrolls both ways.
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
          "flex-row",
          "items-start",
          "gap-6",
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
 * One tree node and everything beneath it. `members` is every agent
 * under this node, each carrying only its remaining path — the node
 * finds itself (an agent whose path ends here), groups the rest by
 * their next segment, and hands each child group down whole.
 * Renders as one TIER cell: its own chip/box on top, the children
 * tier as a left-to-right row beneath it.
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
    <div className={cn("flex", "flex-col", "items-start")}>
      <div
        data-node-kind={kind}
        data-node-name={name}
        className={cn(
          "flex",
          "flex-col",
          "gap-0.5",
          "px-2.5",
          "py-1.5",
          "rounded-sm",
          "border",
          "whitespace-nowrap",
          "select-none",
          // Every node — agent or pure branch — wears the theme
          // orange border and glow; structure stays dashed.
          "border-copper-mid",
          "shadow-[0_0_8px_rgba(217,119,6,0.3)]",
          self === null
            ? // Pure structure: a path segment no agent occupies —
              // its name reads as bright as the agents'.
              cn("border-dashed", "text-info-bright")
            : self.active
              ? cn("bg-copper-warm/10", "text-copper-bright")
              : cn("bg-ground-surface", "text-info-mid"),
        )}
      >
        {self !== null && (
          <AgentTags hierarchy={self.agent_instance_hierarchy} />
        )}
        <span className={cn("text-[11px]")}>{name}</span>
        {self !== null && (
          <div
            className={cn(
              "grid",
              "grid-cols-[auto_auto]",
              "justify-end",
              "gap-x-1.5",
              "gap-y-px",
              "text-[9px]",
              "text-info-dim",
              "tabular-nums",
              // Two right-anchored columns: labels line up at their
              // right edge, dates at theirs.
              "text-right",
            )}
          >
            <span>spawned</span>
            <span>{formatTime(self.created_at)}</span>
            <span>active</span>
            <span>{formatTime(self.last_active_at)}</span>
          </div>
        )}
      </div>
      {children.length > 0 && (
        <>
          {/* Stem from this node down to its children's rail —
              anchored left, under the parent's leading edge. */}
          <div className={cn("w-px", "h-3", "bg-node-border/60", "ml-3")} />
          <div
            className={cn(
              "flex",
              "flex-row",
              "items-start",
              "gap-4",
              "border-t",
              "border-node-border/60",
              "pt-3",
            )}
          >
            {children.map(([child, group]) => (
              <HierarchyNode key={child} name={child} members={group} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

/** One agent's tag chips (live via [`useAgentInstanceTags`] — the
 * hook's dynamic registration mounts and unmounts with this box).
 * While the initial read is in flight, an animated ellipsis stands
 * in; once loaded, one small box per tag (nothing when tagless). */
function AgentTags({ hierarchy }: { hierarchy: string }) {
  const { tags, loading } = useAgentInstanceTags(hierarchy);
  if (loading) {
    return (
      <span
        data-tags-loading
        className={cn(
          "inline-flex",
          "items-center",
          "justify-center",
          "gap-0.5",
          // Text-sized dots at the inherited (chip) font size, so the
          // box doesn't resize when loading resolves into chips.
          "text-info-dim",
        )}
      >
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            className={cn(
              "animate-pulse",
              i === 1 && "[animation-delay:300ms]",
              i === 2 && "[animation-delay:600ms]",
            )}
          >
            .
          </span>
        ))}
      </span>
    );
  }
  if (tags.length === 0) return null;
  return (
    <div
      className={cn(
        "flex",
        "flex-row",
        "flex-wrap",
        "justify-center",
        "gap-1",
      )}
    >
      {tags.map((tag) => (
        <span
          key={tag}
          data-tag={tag}
          className={cn(
            "px-1.5",
            "py-px",
            "rounded-sm",
            "border",
            "border-node-border/70",
            "bg-ground-raised",
            "text-info-mid",
          )}
        >
          {tag}
        </span>
      ))}
    </div>
  );
}

/** RFC3339 → the user's locale, `—` when unknown. */
function formatTime(value: string | null): string {
  if (value === null) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "—" : date.toLocaleString();
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
