import { useLayoutEffect, useMemo, useRef, useState } from "react";
import cn from "classnames";
import { tauriInvoke } from "../lib/tauri";
import {
  agentsTagsApplyExecute,
  agentsTagsRemoveExecute,
  laboratoriesAttachExecute,
  laboratoriesDetachExecute,
  type ViewerTransport,
} from "@objectiveai/sdk";
import { instanceSelector } from "../lib/aih";
import { reportError } from "../lib/errors";
import { daemonExecutor } from "../lib/executor";
import { LoadingDots } from "./LoadingDots";
import { ContextMenu, ContextMenuItem } from "./shared/ContextMenu";
import { OpenTab } from "./shared/OpenTab";
import { describeLastItem } from "./conversationContent";
import type { AgentStatus } from "../hooks/useAgentsInstancesList";
import {
  useLaboratoriesList,
  type LaboratoryStatus,
} from "../hooks/useLaboratoriesList";
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
  // Right-click menu: position (viewport coords) + which pane is
  // showing (the item list, the add-tag input, or the
  // attach-laboratory picker it swaps to).
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const kind = status.active ? "agent-active" : "agent-inactive";
  // A live agent's last-active is implicitly "now" — while active the
  // status row reads `active`; inactive shows the record's timestamp.
  const lastActiveAt = !status.active ? (agent?.last_active_at ?? null) : null;
  // Live-updating relative date; "" (active / no record) renders
  // nothing and schedules nothing.
  const lastActiveAgo = useAgo(lastActiveAt ?? "");
  return (
    <div
      className={cn("flex", "flex-col", "items-stretch", "w-fit")}
      // The menu belongs to the WHOLE node stack — agent box and the
      // attached latest-message box alike.
      onContextMenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
        setMenu({ x: e.clientX, y: e.clientY });
      }}
    >
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
            click). Positioned as a corner tab: pulled up/right through
            this box's padding so its border merges 1px with the box's
            top-right border. */}
        <OpenTab
          dataAttr="data-open-agent"
          onClick={() => onOpen(hierarchy)}
          ariaLabel={`Open ${hierarchy} conversation`}
          className={cn("self-end", "-mt-[7px]", "-mr-[11px]")}
        />
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
              <ActionBadgeRow
                key={tag}
                badge="tag"
                action="remove"
                dataAttr="data-remove-tag"
                onAction={async () => {
                  const executor = await daemonExecutor();
                  const result = await agentsTagsRemoveExecute(executor, {
                    tag,
                  });
                  if ("type" in result && result.type === "error") {
                    throw new Error(
                      typeof result.message === "string"
                        ? result.message
                        : JSON.stringify(result.message),
                    );
                  }
                }}
                onError={(error) => reportError(`remove tag ${tag}`, error)}
              >
                <span data-tag={tag} className={cn("text-[#c3bfbb]")}>
                  {tag}
                </span>
              </ActionBadgeRow>
            ))}
            {/* ATTACHED laboratories only — the active set is
                deliberately not shown here (yet). */}
            {agent.attached_laboratories.map((lab) => (
              <AttachedLaboratoryBadge
                key={lab.id}
                lab={lab}
                hierarchy={hierarchy}
              />
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
      {menu !== null && (
        <ContextMenu position={menu} onClose={() => setMenu(null)}>
          <AgentContextMenuContent
            hierarchy={hierarchy}
            transport={transport}
            attached={agent?.attached_laboratories ?? []}
            onClose={() => setMenu(null)}
          />
        </ContextMenu>
      )}
    </div>
  );
}

/**
 * The add-tag pane the context menu swaps to: a single autofocused
 * input. Enter runs `agents tags apply --agent-instance` targeting
 * the node's `{parent}/{leaf}` and closes on success (the node's tag
 * list refreshes itself through its live instance record — the
 * `tags_changed` trigger); a failure toasts and keeps the input open
 * for a retry. Escape closes via the menu's own handler.
 */
function AddTagInput({
  hierarchy,
  onDone,
}: {
  hierarchy: string;
  onDone: () => void;
}) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const commit = () => {
    const name = value.trim();
    if (!name || busy) {
      return;
    }
    setBusy(true);
    void (async () => {
      try {
        const executor = await daemonExecutor();
        const slash = hierarchy.lastIndexOf("/");
        const result = await agentsTagsApplyExecute(executor, {
          name,
          target: {
            by: "agent_instance",
            agent_instance: hierarchy.slice(slash + 1),
            parent_agent_instance_hierarchy: hierarchy.slice(0, slash),
          },
        });
        if ("type" in result && result.type === "error") {
          throw new Error(
            typeof result.message === "string"
              ? result.message
              : JSON.stringify(result.message),
          );
        }
        onDone();
      } catch (error) {
        reportError(`apply tag to ${hierarchy}`, error);
        setBusy(false);
      }
    })();
  };
  return (
    <div className={cn("px-3", "py-1", "flex", "flex-col", "gap-1")}>
      <span className={cn("text-info-dim")}>tag name</span>
      <input
        autoFocus
        data-menu-tag-input
        value={value}
        disabled={busy}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            commit();
          }
        }}
        className={cn(
          "bg-transparent",
          "border-b",
          "border-copper-mid/60",
          "outline-none",
          "text-info-bright",
          "w-44",
          busy && "opacity-50",
        )}
      />
    </div>
  );
}

/**
 * The agent context menu's CONTENT — mounted only while the menu is
 * open, so the live laboratories-list connection
 * ([`useLaboratoriesList`]) opens on demand, not one-per-agent-node.
 * Owns the pane state (fresh `items` on every open) and computes the
 * ATTACHABLE set: the daemon's laboratories minus the ones already
 * attached to this agent (matched on the full `(id, machine, state)`
 * identity — ids are only unique per machine+state). The "attach
 * laboratory…" item greys out when nothing is attachable.
 */
function AgentContextMenuContent({
  hierarchy,
  transport,
  attached,
  onClose,
}: {
  hierarchy: string;
  transport: ViewerTransport | null;
  attached: { id: string; machine?: string | null; machine_state?: string | null }[];
  onClose: () => void;
}) {
  const [pane, setPane] = useState<"items" | "tag" | "laboratory">("items");
  const laboratories = useLaboratoriesList(transport);
  const available = useMemo(
    () =>
      laboratories.filter(
        (lab) =>
          !attached.some(
            (a) =>
              a.id === lab.id &&
              (a.machine ?? null) === (lab.machine?.id ?? null) &&
              (a.machine_state ?? null) === (lab.machine_state ?? null),
          ),
      ),
    [laboratories, attached],
  );

  if (pane === "tag") {
    return <AddTagInput hierarchy={hierarchy} onDone={onClose} />;
  }
  if (pane === "laboratory") {
    return (
      <AttachLaboratoryPane
        hierarchy={hierarchy}
        laboratories={available}
        onDone={onClose}
      />
    );
  }
  return (
    <>
      <ContextMenuItem
        label="add tag…"
        dataAttr="data-menu-add-tag"
        // A root-level hierarchy has no {parent}/{leaf} split for the
        // apply target — nothing to bind to.
        disabled={!hierarchy.includes("/")}
        onSelect={() => setPane("tag")}
      />
      <ContextMenuItem
        label="attach laboratory…"
        dataAttr="data-menu-attach-laboratory"
        // Nothing left to attach — every live laboratory is already
        // on this agent.
        disabled={available.length === 0}
        onSelect={() => setPane("laboratory")}
      />
    </>
  );
}

/**
 * The attach-laboratory pane the context menu swaps to: the daemon's
 * LIVE laboratories list, one row per laboratory (id, dimmed machine
 * hostname when known). Clicking one runs `laboratories attach`
 * against this node's instance target — pinned to the laboratory's
 * exact (machine, state) when reported — with the row showing
 * [`LoadingDots`] until the command resolves; the attachment row then
 * appears on the node through its live instance record. A failure
 * toasts and re-enables the pane.
 */
function AttachLaboratoryPane({
  hierarchy,
  laboratories,
  onDone,
}: {
  hierarchy: string;
  laboratories: LaboratoryStatus[];
  onDone: () => void;
}) {
  // The (machine, state, id) key of the in-flight attach, if any.
  const [busyKey, setBusyKey] = useState<string | null>(null);

  const attach = (lab: (typeof laboratories)[number], key: string) => {
    if (busyKey !== null) {
      return;
    }
    setBusyKey(key);
    void (async () => {
      try {
        const executor = await daemonExecutor();
        const machine = lab.machine?.id ?? null;
        const result = await laboratoriesAttachExecute(executor, {
          selector: instanceSelector(hierarchy),
          laboratory_id: lab.id,
          // Pin the exact laboratory when its host reported machine
          // identity — ids are only unique per (machine, state).
          ...(machine !== null && lab.machine_state != null
            ? { machine, machine_state: lab.machine_state }
            : {}),
        });
        if ("type" in result && result.type === "error") {
          throw new Error(
            typeof result.message === "string"
              ? result.message
              : JSON.stringify(result.message),
          );
        }
        onDone();
      } catch (error) {
        reportError(`attach laboratory ${lab.id}`, error);
        setBusyKey(null);
      }
    })();
  };

  return (
    <div className={cn("flex", "flex-col")}>
      <span className={cn("px-3", "py-1", "text-info-dim", "select-none")}>
        attach laboratory
      </span>
      {laboratories.length === 0 && (
        <span className={cn("px-3", "py-1", "text-info-dim/60", "italic")}>
          no laboratories
        </span>
      )}
      {laboratories.map((lab) => {
        const key = `${lab.machine?.id ?? ""}\n${lab.machine_state ?? ""}\n${lab.id}`;
        const busy = busyKey === key;
        return (
          <button
            key={key}
            type="button"
            data-menu-laboratory={lab.id}
            disabled={busyKey !== null}
            onClick={() => attach(lab, key)}
            className={cn(
              "flex",
              "flex-row",
              "items-baseline",
              "gap-2",
              "w-full",
              "px-3",
              "py-1",
              "text-left",
              busyKey !== null && !busy
                ? "text-info-dim/60"
                : cn(
                    "cursor-pointer",
                    "hover:bg-copper-warm/10",
                    "hover:text-copper-bright",
                  ),
            )}
          >
            <span className={cn("truncate")}>{lab.id}</span>
            {lab.machine?.hostname != null && (
              <span className={cn("text-info-dim", "truncate")}>
                {lab.machine.hostname}
              </span>
            )}
            {busy && <LoadingDots marker="data-attach-busy" />}
          </button>
        );
      })}
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
          "text-xs",
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
 * row because `useAgo` is a hook (can't run inside the map). The
 * badge is the row's DROPDOWN button — its one action detaches the
 * laboratory from this node's instance target. */
function AttachedLaboratoryBadge({
  lab,
  hierarchy,
}: {
  lab: { id: string; attached_at: string };
  hierarchy: string;
}) {
  const attachedAgo = useAgo(lab.attached_at);
  return (
    <ActionBadgeRow
      badge="laboratory"
      action="detach"
      dataAttr="data-detach-laboratory"
      onAction={async () => {
        const executor = await daemonExecutor();
        const result = await laboratoriesDetachExecute(executor, {
          selector: instanceSelector(hierarchy),
          laboratory_id: lab.id,
        });
        if ("type" in result && result.type === "error") {
          throw new Error(
            typeof result.message === "string"
              ? result.message
              : JSON.stringify(result.message),
          );
        }
      }}
      onError={(error) => reportError(`detach laboratory ${lab.id}`, error)}
    >
      <span data-laboratory={lab.id} className={cn("text-[#c3bfbb]")}>
        {lab.id}
      </span>
      {attachedAgo !== "" && (
        <span
          data-attached-ago
          className={cn(
            // Pushed to the row's right edge (the row stretches to
            // the box's width).
            "ml-auto",
            "text-xs",
            "text-info-mid",
            "tabular-nums",
          )}
        >
          {attachedAgo}
        </span>
      )}
    </ActionBadgeRow>
  );
}

/**
 * A [`BadgeRow`] whose badge chip is a BUTTON: clicking it drops a
 * one-item menu (`remove` / `detach`); selecting the action closes
 * the menu and shows [`LoadingDots`] in the chip until the command
 * resolves — the row itself disappears via the node's live instance
 * record on success (never optimistically), and a failure reports
 * through `onError` and restores the chip.
 */
function ActionBadgeRow({
  badge,
  action,
  dataAttr,
  onAction,
  onError,
  children,
}: {
  badge: string;
  action: string;
  dataAttr?: string;
  onAction: () => Promise<void>;
  onError: (error: unknown) => void;
  children?: React.ReactNode;
}) {
  const [menu, setMenu] = useState<{ x: number; y: number } | null>(null);
  const [busy, setBusy] = useState(false);
  return (
    <div
      className={cn(
        "flex",
        "flex-row",
        "items-center",
        "gap-1.5",
        // Full box width (unlike BadgeRow's self-start) so trailing
        // metadata can right-align via ml-auto.
        "self-stretch",
        "mt-1",
        "first:mt-0",
        "text-sm",
      )}
    >
      <button
        type="button"
        {...(dataAttr !== undefined ? { [dataAttr]: true } : {})}
        aria-label={`${badge} actions`}
        disabled={busy}
        onClick={(e) => {
          const rect = e.currentTarget.getBoundingClientRect();
          setMenu({ x: rect.left, y: rect.bottom + 2 });
        }}
        className={cn(
          "px-1.5",
          "py-px",
          "rounded-sm",
          "border",
          "text-xs",
          "border-copper-mid/70",
          "bg-copper-warm/10",
          "text-copper-bright",
          busy
            ? "opacity-70"
            : cn(
                "cursor-pointer",
                "hover:border-copper-hot",
                "hover:text-copper-hot",
              ),
        )}
      >
        {busy ? <LoadingDots marker="data-badge-busy" /> : badge}
      </button>
      {children}
      {menu !== null && (
        <ContextMenu position={menu} onClose={() => setMenu(null)}>
          <ContextMenuItem
            label={action}
            dataAttr="data-badge-action"
            onSelect={() => {
              setMenu(null);
              setBusy(true);
              void onAction()
                .catch(onError)
                .finally(() => setBusy(false));
            }}
          />
        </ContextMenu>
      )}
    </div>
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
