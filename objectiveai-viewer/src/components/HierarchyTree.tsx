import { useLayoutEffect, useRef, useState } from "react";
import cn from "classnames";
import { tauriInvoke } from "../lib/tauri";
import type { ViewerTransport } from "@objectiveai/sdk";
import { LoadingDots } from "./LoadingDots";
import { describeLastItem } from "./conversationContent";
import type { AgentStatus } from "../hooks/useAgentsInstancesList";
import {
  useAgentInstance,
  type ConversationBlock,
} from "../hooks/useAgentInstance";
import {
  useAgentDefinition,
  type AgentDefinition,
} from "../hooks/useAgentDefinition";
import { useAgo } from "../hooks/useAgo";
import { useOrientation } from "../hooks/useOrientation";

/** One agent scoped under a tree node: the path segments REMAINING
 * below that node, plus the agent itself. `rest` empty means the
 * node IS this agent. */
interface ScopedAgent {
  rest: readonly string[];
  status: AgentStatus;
}

/**
 * The whole-body agent hierarchy: every known agent's instance
 * hierarchy (from App's agents-list connection) split on `/` into an
 * org-chart tree — TIERS stack top-down, siblings flow left-to-right
 * on the same level (`fizz/foo/bar` + `fizz/foo/buzz` → three tiers,
 * with `bar` and `buzz` side by side under `foo`). Everything anchors
 * LEFT: a parent sits at its subtree's leading edge, with the first
 * child directly below it. Each node shows ONLY its own segment —
 * never a full hierarchy. Each AGENT node opens its own
 * `/agents/instances/{*aih}` connection ([`useAgentInstance`]) for
 * its tags, timestamps, and latest conversation item; branches with
 * no corresponding agent render dashed as pure structure. Scrolls
 * both ways.
 */
export function HierarchyTree({
  transport,
  agents,
  zoom = 1,
}: {
  transport: ViewerTransport | null;
  agents: AgentStatus[];
  /** Canvas zoom factor from the footer slider. Applied as the CSS
   * `zoom` property (Chromium reflows layout under it, so the scroll
   * extents stay correct — unlike `transform: scale`). */
  zoom?: number;
}) {
  // Canvas orientation (footer toggle): vertical = tiers top-down,
  // horizontal = the transpose. Every scaffold axis below derives
  // from this.
  const orientation = useOrientation();
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
        style={{ zoom }}
        className={cn(
          "min-w-max",
          "p-6",
          "font-mono",
          "text-xs",
          "flex",
          orientation === "vertical" ? "flex-row" : "flex-col",
          "items-start",
          "gap-6",
        )}
      >
        {roots.map(([name, members]) => (
          <HierarchyNode
            key={name}
            name={name}
            members={members}
            transport={transport}
            onOpen={openAgentWindow}
          />
        ))}
      </div>
    </div>
    </>
  );
}

/** Open (or focus) the agent's conversation WINDOW — a real Tauri
 * window on the `agent.html` entry, created by the Rust shell. */
function openAgentWindow(hierarchy: string): void {
  void tauriInvoke("open_agent_window", { aih: hierarchy });
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
  transport,
  onOpen,
}: {
  name: string;
  members: ScopedAgent[];
  transport: ViewerTransport | null;
  onOpen: (hierarchy: string) => void;
}) {
  const orientation = useOrientation();
  const vertical = orientation === "vertical";
  // The agent AT this node, if any (hierarchies are unique — at most
  // one member terminates here).
  const self =
    members.find((member) => member.rest.length === 0)?.status ?? null;
  const children = groupByHead(
    members.filter((member) => member.rest.length > 0),
  );
  return (
    <div className={cn("flex", vertical ? "flex-col" : "flex-row", "items-start")}>
      {self !== null ? (
        <AgentNode
          name={name}
          status={self}
          transport={transport}
          onOpen={onOpen}
        />
      ) : (
        <div
          data-node-kind="branch"
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
            "border-copper-mid",
            "shadow-[0_0_8px_rgba(217,119,6,0.3)]",
            // Pure structure: a path segment no agent occupies —
            // its name reads as bright as the agents'.
            "border-dashed",
            "text-info-bright",
          )}
        >
          <span className={cn("text-sm")}>{name}</span>
        </div>
      )}
      {children.length > 0 && (
        <>
          {/* Stem from this node toward its children's rail —
              anchored at the parent's leading edge (12px in from the
              descent-side corner). */}
          <div
            className={cn(
              vertical ? cn("w-px", "h-3", "ml-3") : cn("h-px", "w-3", "mt-3"),
              "bg-copper-hot",
            )}
          />
          <div
            className={cn(
              "flex",
              vertical ? "flex-row" : "flex-col",
              "items-start",
            )}
          >
            {children.map(([child, group], i) => {
              const first = i === 0;
              const last = i === children.length - 1;
              return (
                <div
                  key={child}
                  className={cn(
                    "flex",
                    vertical ? "flex-col" : "flex-row",
                    "items-stretch",
                  )}
                >
                  {/* The rail, built per-child so it CAPS at the
                      first and last drop points: a leading piece up
                      to this child's stem (skipped on the first
                      child) and a trailing piece onward to the next
                      sibling (skipped on the last). A single child
                      renders neither — just the straight stem. */}
                  {children.length > 1 && (
                    <div
                      className={cn(
                        "flex",
                        vertical ? "flex-row" : "flex-col",
                      )}
                    >
                      <div
                        className={cn(
                          vertical ? cn("h-px", "w-3") : cn("w-px", "h-3"),
                          "shrink-0",
                          !first && "bg-copper-hot",
                        )}
                      />
                      <div
                        className={cn(
                          vertical ? "h-px" : "w-px",
                          "flex-1",
                          !last && "bg-copper-hot",
                        )}
                      />
                    </div>
                  )}
                  <div
                    className={cn(
                      "flex",
                      vertical ? "flex-col" : "flex-row",
                      "items-start",
                      // Sibling spacing lives INSIDE the cell so the
                      // rail's trailing piece spans the whole gap.
                      !last && (vertical ? "pr-4" : "pb-4"),
                    )}
                  >
                    {/* Stem from the rail INTO this child. */}
                    <div
                      className={cn(
                        vertical
                          ? cn("w-px", "h-3", "ml-3")
                          : cn("h-px", "w-3", "mt-3"),
                        "bg-copper-hot",
                      )}
                    />
                    <HierarchyNode
                      name={child}
                      members={group}
                      transport={transport}
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

/**
 * One AGENT node's card pair: the agent box (tags + definition +
 * last-active) and, attached below it, the latest conversation item.
 * Owns this agent's `/agents/instances/{*aih}` connection — tags and
 * timestamps come from its status record, the message body from its
 * conversation's last block. The `active` flag comes from the LIST
 * connection (`status.active`) so the whole tree flips without every
 * node owning a record yet.
 */
function AgentNode({
  name,
  status,
  transport,
  onOpen,
}: {
  name: string;
  status: AgentStatus;
  transport: ViewerTransport | null;
  onOpen: (hierarchy: string) => void;
}) {
  const hierarchy = status.agent_instance_hierarchy;
  const { agent, lastBlock } = useAgentInstance(transport, hierarchy);
  const kind = status.active ? "agent-active" : "agent-inactive";
  // A live agent's last-active is implicitly "now" — while active the
  // status row reads `active`; inactive shows the record's timestamp.
  const lastActiveAt = !status.active ? (agent?.last_active_at ?? null) : null;
  // Live-updating relative date; "" (active / no record) renders
  // nothing and schedules nothing.
  const lastActiveAgo = useAgo(lastActiveAt ?? "");
  return (
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
          "border-copper-mid",
          "shadow-[0_0_8px_rgba(217,119,6,0.3)]",
          // Active and inactive boxes look IDENTICAL — the status
          // row's dot (same indicator as the footer's) is the only
          // visual difference.
          "bg-ground-surface",
          "text-info-mid",
        )}
      >
        {/* The EXPLICIT opener, on its own line above the instance
            chip — the only way to open the conversation (no whole-box
            click). */}
        <button
          type="button"
          data-open-agent
          onClick={() => onOpen(hierarchy)}
          aria-label={`Open ${hierarchy} conversation`}
          className={cn(
            "self-end",
            // A corner TAB: pulled up/right through the box padding so
            // its border merges 1px with the box's top-right border —
            // rounded only where it matches the box corner (tr) and
            // where it faces the content (bl).
            "-mt-[7px]",
            "-mr-[11px]",
            "rounded-tr-sm",
            "rounded-bl-sm",
            "rounded-tl-none",
            "rounded-br-none",
            "px-1.5",
            "py-px",
            "border",
            "border-copper-mid",
            "bg-copper-warm/10",
            "text-copper-bright",
            "text-xs",
            "hover:border-copper-hot",
            "hover:text-copper-hot",
            "cursor-pointer",
          )}
        >
          open ↗
        </button>
        {agent === null ? (
          <LoadingDots marker="data-tags-loading" />
        ) : (
          <>
            <BadgeRow badge="instance">
              <span data-tag-aih className={cn("text-[#c3bfbb]")}>
                {name}
              </span>
            </BadgeRow>
            {agent.tags.map((tag) => (
              <BadgeRow key={tag} badge="tag">
                <span data-tag={tag} className={cn("text-[#c3bfbb]")}>
                  {tag}
                </span>
              </BadgeRow>
            ))}
            {/* ATTACHED laboratories only — the active set is
                deliberately not shown here (yet). */}
            {agent.attached_laboratories.map((lab) => (
              <AttachedLaboratoryBadge key={lab.id} lab={lab} />
            ))}
          </>
        )}
        <AgentDefinitionView hierarchy={hierarchy} />
        {(status.active || lastActiveAt !== null) && (
          <span
            data-last-active
            className={cn(
              "self-end",
              "flex",
              "items-center",
              "gap-1.5",
              "text-xs",
              "text-info-mid",
              "tabular-nums",
            )}
          >
            {/* The footer's exact active indicator. */}
            <span
              className={cn(
                "w-1.5",
                "h-1.5",
                "rounded-full",
                status.active
                  ? cn("bg-copper-hot", "animate-pulse")
                  : "bg-info-dim",
              )}
            />
            {status.active ? "active" : lastActiveAgo}
          </span>
        )}
      </div>
      {lastBlock !== null && <LastItemView block={lastBlock} />}
    </div>
  );
}

/** The agent's most recent conversation item, in its own box below
 * the agent's — with an indicator chip naming exactly what is being
 * shown (`assistant · text`, `tool call · <name>`, …). Live from the
 * agent's own `/agents/instances/{*aih}` connection. */
function LastItemView({ block }: { block: ConversationBlock }) {
  const { label, body } = describeLastItem(block);
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
  }, [block, expanded]);

  if (body === null) return null;
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
        "text-xs",
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
      {/* The indicator: exactly what kind of item is shown below. */}
      <span
        data-last-item-kind
        className={cn(
          "self-start",
          "mb-1",
          "px-1.5",
          "py-px",
          "rounded-sm",
          "border",
          "border-copper-mid/70",
          "bg-copper-warm/10",
          "text-copper-bright",
          "text-xs",
        )}
      >
        {label}
      </span>
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
            container clips it. */}
        <div ref={contentRef} className={cn("shrink-0")}>
          {body}
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
        "text-sm",
        "text-copper-bright",
      )}
    >
      <span
        className={cn(
          "px-1.5",
          "py-px",
          "text-xs",
          "bg-copper-warm/10",
          "rounded-br-sm",
        )}
      >
        inline
      </span>
      <pre
        data-agent-definition
        className={cn(
          "text-sm",
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

/** One attached-laboratory row: the lab id plus a live "ago" for when
 * the attachment was made (`attached_at`, RFC3339). A component per
 * row because `useAgo` is a hook (can't run inside the map). */
function AttachedLaboratoryBadge({
  lab,
}: {
  lab: { id: string; attached_at: string };
}) {
  const attachedAgo = useAgo(lab.attached_at);
  return (
    <BadgeRow badge="laboratory">
      <span data-laboratory={lab.id} className={cn("text-[#c3bfbb]")}>
        {lab.id}
      </span>
      {attachedAgo !== "" && (
        <span
          data-attached-ago
          className={cn("text-xs", "text-info-mid", "tabular-nums")}
        >
          {attachedAgo}
        </span>
      )}
    </BadgeRow>
  );
}

/** The uniform badge-plus-value row every definition-ish line uses:
 * a copper badge chip naming the KIND (`instance` / `tag` / `client` /
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
        "text-sm",
      )}
    >
      <span
        className={cn(
          "px-1.5",
          "py-px",
          "rounded-sm",
          "border",
          "text-xs",
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

/** A remote definition as a badge + link row: a kind badge
 * (`client` / `github` / `mock`) and `owner/repository` — a
 * hyperlink for `client` (opens the local agents folder via the
 * Rust `open_agent_remote` command) and `github` (opens the repo,
 * or `/tree/<commit>` when pinned — the short sha shown inline);
 * plain text for `mock`. */
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
