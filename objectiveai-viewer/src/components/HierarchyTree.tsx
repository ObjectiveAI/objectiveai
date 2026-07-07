import { useLayoutEffect, useRef, useState } from "react";
import cn from "classnames";
import { tauriInvoke } from "../lib/tauri";
import { ConversationModal } from "./ConversationModal";
import { LoadingDots } from "./LoadingDots";
import { Markdown } from "./Markdown";
import { useAgents, type AgentStatus } from "../hooks/useAgents";
import {
  useAgentDefinition,
  type AgentDefinition,
} from "../hooks/useAgentDefinition";
import { useAgentInstanceTags } from "../hooks/useAgentInstanceTags";
import { useAgentLatestText } from "../hooks/useAgentLatestText";
import { formatAgo } from "../lib/formatAgo";

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
  const [openHierarchy, setOpenHierarchy] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const dragRef = useRef<{
    x: number;
    y: number;
    left: number;
    top: number;
  } | null>(null);
  const [dragging, setDragging] = useState(false);

  // MOUSE-only drag-to-pan. Touch/pen pointers are ignored entirely —
  // mobile keeps its native scroller untouched.
  const onPointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
    if (e.pointerType !== "mouse" || e.button !== 0) return;
    // Don't hijack interactive elements or selectable text.
    const target = e.target as Element;
    if (target.closest("button, a, [data-latest-text], pre")) return;
    const el = containerRef.current;
    if (el === null) return;
    dragRef.current = {
      x: e.clientX,
      y: e.clientY,
      left: el.scrollLeft,
      top: el.scrollTop,
    };
    // NO capture yet: capturing on pointerdown retargets the
    // eventual click to the container, swallowing box clicks. The
    // move handler captures once a real pan engages.
  };
  const onPointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
    const drag = dragRef.current;
    const el = containerRef.current;
    if (drag === null || el === null) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    if (!dragging && Math.abs(dx) + Math.abs(dy) > 3) {
      setDragging(true);
      try {
        el.setPointerCapture(e.pointerId);
      } catch {
        // jsdom / detached — capture is best-effort.
      }
    }
    el.scrollLeft = drag.left - dx;
    el.scrollTop = drag.top - dy;
  };
  const onPointerEnd = (e: React.PointerEvent<HTMLDivElement>) => {
    dragRef.current = null;
    setDragging(false);
    const el = containerRef.current;
    if (el !== null) {
      try {
        el.releasePointerCapture(e.pointerId);
      } catch {
        // Never captured — fine.
      }
    }
  };

  const roots = groupByHead(
    agents.map((status) => ({
      rest: status.agent_instance_hierarchy.split("/"),
      status,
    })),
  );
  return (
    <>
    <div
      ref={containerRef}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerEnd}
      onPointerCancel={onPointerEnd}
      className={cn(
        "absolute",
        "inset-0",
        "overflow-auto",
        // Regular cursor at rest; grabby only while actually
        // panning, with NO text selection anywhere beneath
        // (descendants' select-text included).
        dragging && cn("cursor-grabbing", "[&_*]:select-none"),
      )}
    >
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
          <HierarchyNode
            key={name}
            name={name}
            members={members}
            onOpen={setOpenHierarchy}
          />
        ))}
      </div>
    </div>
    {openHierarchy !== null && (
      <ConversationModal
        hierarchy={openHierarchy}
        onClose={() => setOpenHierarchy(null)}
      />
    )}
    </>
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
  onOpen,
}: {
  name: string;
  members: ScopedAgent[];
  onOpen: (hierarchy: string) => void;
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
        role={self !== null ? "button" : undefined}
        onClick={
          self === null
            ? undefined
            : (e) => {
                // The definition hyperlinks keep their own clicks.
                if ((e.target as Element).closest("button,a")) return;
                onOpen(self.agent_instance_hierarchy);
              }
        }
        className={cn(
          self !== null && cn("cursor-pointer", "hover:border-copper-hot"),
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
        {self !== null && self.last_active_at !== null && (
          <span
            data-last-active
            className={cn(
              "self-end",
              "text-[9px]",
              "text-info-mid",
              "tabular-nums",
            )}
          >
            {formatAgo(self.last_active_at)}
          </span>
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
                    <HierarchyNode
                      name={child}
                      members={group}
                      onOpen={onOpen}
                    />
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
  const contentRef = useRef<HTMLDivElement | null>(null);

  // Detect ACTUAL hidden content: only then does the toggle appear.
  // Note scrollHeight can't see justify-end's TOP overflow, so the
  // measure compares the inner content's natural height against the
  // clip window instead.
  useLayoutEffect(() => {
    const clip = clipRef.current;
    const content = contentRef.current;
    if (clip === null || content === null) return;
    const measure = () => {
      setClipped(content.offsetHeight > clip.clientHeight + 1);
    };
    measure();
    // The box's width settles as its siblings (tags, definition)
    // load in — a narrow first paint wraps the text taller than it
    // will really be, so keep re-measuring on any size change.
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(clip);
    observer.observe(content);
    return () => {
      observer.disconnect();
    };
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
        // Anchor for the border-straddling toggle pill; extra bottom
        // room so text clears the pill's upper half.
        "relative",
        (clipped || expanded) && "pb-3.5",
      )}
    >
      <div
        ref={clipRef}
        className={cn(
          "break-words",
          "select-text",
          "leading-4",
          // Collapsed: cap the preview visually, showing the most
          // recent content — the body bottom-anchors in a fixed
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
        {/* shrink-0 keeps the natural height measurable while the
            container clips it. Light element styling for the
            rendered markdown. */}
        <div ref={contentRef} className={cn("shrink-0")}>
          <Markdown>{text}</Markdown>
        </div>
      </div>
      {(clipped || expanded) && (
        <button
          type="button"
          data-expand-text
          onClick={() => setExpanded((v) => !v)}
          aria-label={expanded ? "Collapse message" : "Expand message"}
          className={cn(
            // An oval SPLITTING the bottom border: centered on the
            // border line, opaque ground fill masking the border
            // beneath it.
            "absolute",
            "left-1/2",
            "-translate-x-1/2",
            "-bottom-2.5",
            "px-2.5",
            "py-1.5",
            "rounded-full",
            "border",
            "border-copper-mid",
            "bg-ground",
            "text-copper-mid",
            "hover:text-copper-bright",
            "cursor-pointer",
          )}
        >
          {/* A flat, WIDE chevron — much wider than tall. */}
          <svg
            width="18"
            height="7"
            viewBox="0 0 18 7"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            {expanded ? (
              <path d="M2 5L9 2L16 5" />
            ) : (
              <path d="M2 2L9 5L16 2" />
            )}
          </svg>
        </button>
      )}
    </div>
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
      <BadgeRow badge="instance">
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
