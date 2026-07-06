import { useLayoutEffect, useRef, useState } from "react";
import cn from "classnames";
import { tauriInvoke } from "../lib/tauri";
import { useAgents, type AgentStatus } from "../hooks/useAgents";
import {
  useAgentDefinition,
  type AgentDefinition,
} from "../hooks/useAgentDefinition";
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
      {/* The card pair: the agent box sets the width; the attached
          message box stretches to match it exactly. */}
      <div className={cn("flex", "flex-col", "items-stretch", "w-fit")}>
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
          // Square the bottom IFF a message box is attached below —
          // the message box carries the bottom rounding then.
          "[&:has(+[data-latest-text])]:rounded-b-none",
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
        {self !== null ? (
          // The AIH segment rides the tag row as just another chip —
          // it appears with the tags, post-load.
          <AgentTags hierarchy={self.agent_instance_hierarchy} name={name} />
        ) : (
          <span className={cn("text-[11px]")}>{name}</span>
        )}
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
              "text-info-mid",
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
      </div>
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
  const [expanded, setExpanded] = useState(false);
  const [clipped, setClipped] = useState(false);
  const clipRef = useRef<HTMLDivElement | null>(null);

  // Detect ACTUAL hidden content: only then does the toggle appear.
  useLayoutEffect(() => {
    const el = clipRef.current;
    if (el === null) return;
    setClipped(el.scrollHeight > el.clientHeight + 1);
  }, [text, expanded]);

  if (text === null) return null;
  return (
    <div
      data-latest-text
      className={cn(
        // Attached to the agent box above: borders overlap 1px and
        // share the agent's copper-mid, so the pair reads as one
        // connected unit.
        "-mt-px",
        // Zero preferred width + min-w-full: the wrapper (sized by
        // the agent box) dictates this box's width exactly.
        "w-0",
        "min-w-full",
        "px-2.5",
        "py-1.5",
        "rounded-sm",
        "rounded-t-none",
        "border",
        "border-copper-mid",
        "text-[10px]",
        // Between info-bright (#d6d3d1) and info-mid (#a8a29e).
        "text-[#c3bfbb]",
        "text-left",
        "flex",
        "flex-col",
      )}
    >
      <div
        ref={clipRef}
        className={cn(
          "whitespace-pre-wrap",
          "break-words",
          "select-text",
          "leading-4",
          // Collapsed: cap at 5 VISUAL lines, showing the most
          // recent ones — the text bottom-anchors in a fixed
          // max-height clip, so overflow disappears off the TOP.
          !expanded &&
            cn(
              "max-h-[77px]",
              "overflow-hidden",
              "flex",
              "flex-col",
              "justify-end",
            ),
        )}
      >
        {text}
      </div>
      {(clipped || expanded) && (
        <button
          type="button"
          data-expand-text
          onClick={() => setExpanded((v) => !v)}
          aria-label={expanded ? "Collapse message" : "Expand message"}
          className={cn(
            "self-center",
            "mt-0.5",
            "leading-none",
            "text-copper-mid",
            "hover:text-copper-bright",
            "cursor-pointer",
          )}
        >
          {expanded ? "\u25b4" : "\u25be"}
        </button>
      )}
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

/** The recorded agent definition. Remotes render as a badge + link
 * row ([`RemoteDefinition`]); inline specs as full pretty-printed
 * JSON. Its own loading ellipsis; nothing when the registry knows
 * nothing. */
function AgentDefinitionView({ hierarchy }: { hierarchy: string }) {
  const { agent, loading } = useAgentDefinition(hierarchy);
  if (loading) {
    return <LoadingDots marker="data-definition-loading" />;
  }
  if (agent == null) return null;
  if ("remote" in agent) {
    return <RemoteDefinition remote={agent} />;
  }
  return (
    // A regular badge chip, just BIGGER: identical border, padding,
    // and tint — the label meets the top-left exactly like every
    // other badge, and the box simply extends to hold the JSON.
    <div
      className={cn(
        "flex",
        "flex-col",
        "items-start",
        "self-start",
        "mt-1",
        "first:mt-0",
        "rounded-sm",
        "border",
        "border-copper-mid/70",
        // The label's tint clips to the border at the top-left.
        "overflow-hidden",
        "text-[11px]",
        "text-copper-bright",
      )}
    >
      <span
        className={cn(
          "px-1.5",
          "py-px",
          "bg-copper-warm/10",
          "rounded-br-sm",
        )}
      >
        inline
      </span>
      <pre
        data-agent-definition
        className={cn(
          "text-[9px]",
          "text-[#c3bfbb]",
          "text-left",
          "whitespace-pre",
          "leading-snug",
          "px-1.5",
          "py-1",
        )}
      >
        {formatDefinition(agent)}
      </pre>
    </div>
  );
}

/** The remote variants of the definition union. */
type RemoteDefinitionValue = Extract<AgentDefinition, { remote: string }>;

/** A remote definition as a badge + link row: a kind badge
 * (`client` / `github` / `mock`) and `owner/repository` — a
 * hyperlink for `client` (opens the local agents folder via the
 * Rust `open_agent_remote` command) and `github` (opens the repo,
 * or `/tree/<commit>` when pinned — the short sha shown inline);
 * plain text for `mock`. */
/** The uniform badge-plus-value row every definition-ish line uses:
 * a copper badge chip naming the KIND (`aih` / `tag` / `client` /
 * `github` / `mock`) followed by the value. */
function BadgeRow({
  badge,
  children,
  ...rest
}: {
  badge: string;
  children?: React.ReactNode;
} & Record<string, unknown>) {
  return (
    <div
      {...rest}
      className={cn(
        "flex",
        "flex-row",
        "items-center",
        "gap-1.5",
        "self-start",
        // Breathing room between badge rows (the first hugs the
        // box's own padding).
        "mt-1",
        "first:mt-0",
        "text-[11px]",
      )}
    >
      <span
        className={cn(
          "px-1.5",
          "py-px",
          "rounded-sm",
          "border",
          "border-copper-mid/70",
          "bg-copper-warm/10",
          "text-copper-bright",
        )}
      >
        {badge}
      </span>
      {children}
    </div>
  );
}

function RemoteDefinition({ remote }: { remote: RemoteDefinitionValue }) {
  // Mock remotes carry a bare `name`; client/github carry
  // owner/repository coordinates.
  const label =
    remote.remote === "mock"
      ? remote.name
      : `${remote.owner}/${remote.repository}`;
  const commit =
    "commit" in remote && typeof remote.commit === "string"
      ? remote.commit
      : null;
  const open =
    remote.remote === "client"
      ? () =>
          void tauriInvoke("open_agent_remote", {
            owner: remote.owner,
            repository: remote.repository,
          })
      : remote.remote === "github"
        ? () =>
            void tauriInvoke("open_url", {
              url:
                commit === null
                  ? `https://github.com/${remote.owner}/${remote.repository}`
                  : `https://github.com/${remote.owner}/${remote.repository}/tree/${commit}`,
            })
        : null;
  return (
    <BadgeRow badge={remote.remote} data-agent-remote={remote.remote}>
      {open !== null ? (
        <button
          type="button"
          data-remote-link
          onClick={open}
          className={cn(
            "text-[#c3bfbb]",
            "underline",
            "hover:text-copper-bright",
            "cursor-pointer",
          )}
        >
          {label}
        </button>
      ) : (
        <span className={cn("text-[#c3bfbb]")}>{label}</span>
      )}
      {remote.remote === "github" && commit !== null && (
        <span className={cn("text-info-dim")}>@{commit.slice(0, 7)}</span>
      )}
    </BadgeRow>
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
function AgentTags({ hierarchy, name }: { hierarchy: string; name: string }) {
  const { tags, loading } = useAgentInstanceTags(hierarchy);
  if (loading) {
    return <LoadingDots marker="data-tags-loading" />;
  }
  // Borderless, background-less values in the message body's tone —
  // the rows ARE the remote row's component, badged by kind.
  const value = cn("text-[#c3bfbb]");
  return (
    <>
      <BadgeRow badge="aih">
        <span data-tag-aih className={value}>
          {name}
        </span>
      </BadgeRow>
      {tags.map((tag) => (
        <BadgeRow key={tag} badge="tag">
          <span data-tag={tag} className={value}>
            {tag}
          </span>
        </BadgeRow>
      ))}
    </>
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
