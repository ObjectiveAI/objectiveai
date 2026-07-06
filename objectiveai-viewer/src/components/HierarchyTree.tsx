import cn from "classnames";
import { useAgents, type AgentStatus } from "../hooks/useAgents";
import { useAgentDefinition } from "../hooks/useAgentDefinition";
import { useAgentInstanceTags } from "../hooks/useAgentInstanceTags";
import { useAgentLatestText } from "../hooks/useAgentLatestText";

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
          <AgentDefinitionView hierarchy={self.agent_instance_hierarchy} />
        )}
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
      {self !== null && (
        <AgentLatestTextView
          hierarchy={self.agent_instance_hierarchy}
          active={self.active}
        />
      )}
      {children.length > 0 && (
        <>
          {/* Stem from this node down to its children's rail —
              anchored left, under the parent's leading edge. */}
          <div
            className={cn(
              "w-px",
              "h-3",
              "ml-3",
              "bg-copper-hot",
            )}
          />
          <div className={cn("flex", "flex-row", "items-start")}>
            {children.map(([child, group], i) => {
              const first = i === 0;
              const last = i === children.length - 1;
              return (
                <div
                  key={child}
                  className={cn("flex", "flex-col", "items-stretch")}
                >
                  {/* The rail, built per-child so it CAPS at the
                      first and last drop points: a left piece up to
                      this child's stem (skipped on the first child)
                      and a right piece onward to the next sibling
                      (skipped on the last). A single child renders
                      neither — just the straight vertical. */}
                  {children.length > 1 && (
                    <div className={cn("flex", "flex-row")}>
                      <div
                        className={cn(
                          "h-px",
                          "w-3",
                          "shrink-0",
                          !first && "bg-copper-hot",
                        )}
                      />
                      <div
                        className={cn(
                          "h-px",
                          "flex-1",
                          !last && "bg-copper-hot",
                        )}
                      />
                    </div>
                  )}
                  <div
                    className={cn(
                      "flex",
                      "flex-col",
                      "items-start",
                      // Sibling spacing lives INSIDE the cell so the
                      // rail's right piece spans the whole gap.
                      !last && "pr-4",
                    )}
                  >
                    {/* Stem from the rail down INTO this child. */}
                    <div
                      className={cn("w-px", "h-3", "ml-3", "bg-copper-hot")}
                    />
                    <HierarchyNode name={child} members={group} />
                  </div>
                </div>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}

/** The agent's most recent assistant TEXT, in its own box below the
 * agent's — absent entirely when the agent has written none. Live
 * via [`useAgentLatestText`]: refreshed on active flips and polled
 * every 60s while active. */
function AgentLatestTextView({
  hierarchy,
  active,
}: {
  hierarchy: string;
  active: boolean;
}) {
  const text = useAgentLatestText(hierarchy, active);
  if (text === null) return null;
  return (
    <div
      data-latest-text
      className={cn(
        "mt-1",
        "max-w-72",
        "px-2.5",
        "py-1.5",
        "rounded-sm",
        "border",
        "border-copper-mid/60",
        "text-[10px]",
        "text-info-mid",
        "text-left",
        "whitespace-pre-wrap",
        "break-words",
        "select-text",
      )}
    >
      {text}
    </div>
  );
}

/** The staggered-pulse ellipsis every per-box loading state shows.
 * Text-sized dots at the inherited font size, so boxes don't resize
 * when loading resolves into content. */
function LoadingDots({ marker }: { marker: string }) {
  return (
    <span
      {...{ [marker]: true }}
      className={cn(
        "inline-flex",
        "items-center",
        "justify-center",
        "gap-0.5",
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

/** The recorded agent definition (remote path or inline spec), shown
 * as full pretty-printed JSON — whatever shape it is. Its own
 * loading ellipsis; nothing when the registry knows nothing. */
function AgentDefinitionView({ hierarchy }: { hierarchy: string }) {
  const { agent, loading } = useAgentDefinition(hierarchy);
  if (loading) {
    return <LoadingDots marker="data-definition-loading" />;
  }
  if (agent == null) return null;
  return (
    <pre
      data-agent-definition
      className={cn(
        "text-[9px]",
        "text-info-bright",
        "text-left",
        "whitespace-pre",
        "leading-snug",
      )}
    >
      {formatDefinition(agent)}
    </pre>
  );
}

/** Pretty-print the definition, omitting every null-valued key (at
 * any depth) — nulls are wire noise, not information. */
function formatDefinition(agent: unknown): string {
  return JSON.stringify(
    agent,
    (_key, value: unknown) => (value === null ? undefined : value),
    2,
  );
}

/** One agent's tag chips (live via [`useAgentInstanceTags`] — the
 * hook's dynamic registration mounts and unmounts with this box).
 * While the initial read is in flight, an animated ellipsis stands
 * in; once loaded, one small box per tag (nothing when tagless). */
function AgentTags({ hierarchy }: { hierarchy: string }) {
  const { tags, loading } = useAgentInstanceTags(hierarchy);
  if (loading) {
    return <LoadingDots marker="data-tags-loading" />;
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
