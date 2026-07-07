import { useEffect } from "react";
import cn from "classnames";
import type { DaemonConnection } from "../lib/daemon";
import {
  useAgentInstance,
  type ConversationBlock,
  type ConversationRow,
} from "../hooks/useAgentInstance";
import { blockScope, describeRow, KindChip } from "./conversationContent";
import { formatAgo } from "../lib/formatAgo";
import { LoadingDots } from "./LoadingDots";

/**
 * The per-agent conversation popup: near-fullscreen panel over a dim
 * backdrop, closable via the X, Escape, or a backdrop click.
 *
 * Renders the agent's WHOLE conversation from its own
 * `/agents/instances/{*aih}` connection ([`useAgentInstance`] — the
 * same listener the tree's agent nodes use): the DB snapshot replays
 * on connect, then live rows stream in as they occur, each block's
 * content INLINE (no per-part fetches). Every block is a card headed
 * by an indicator chip naming exactly what it is — `assistant`,
 * `tool call · <name>` (arguments as formatted JSON), `user`,
 * `notification` (with sender + queued time), `choices`, `vote` — and
 * its parts rendered per kind by the shared conversation renderers.
 */
export function ConversationModal({
  connection,
  hierarchy,
  onClose,
}: {
  connection: DaemonConnection | null;
  hierarchy: string;
  onClose: () => void;
}) {
  const { blocks, live } = useAgentInstance(connection, hierarchy);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => {
      window.removeEventListener("keydown", handler);
    };
  }, [onClose]);

  return (
    <div
      data-conversation-backdrop
      onClick={onClose}
      className={cn(
        // Absolute within the main pane's relative root — the popup
        // is CONSTRAINED to the tab's content area (like plugin
        // panes) and never eclipses the tab bar or status bar.
        "absolute",
        "inset-0",
        "z-50",
        "bg-ground/80",
        "backdrop-blur-sm",
      )}
    >
      <div
        data-conversation-modal
        onClick={(e) => {
          e.stopPropagation();
        }}
        className={cn(
          "absolute",
          "inset-4",
          "flex",
          "flex-col",
          "overflow-hidden",
          "rounded-md",
          "border",
          "border-copper-mid",
          "bg-ground",
          "shadow-[0_0_16px_rgba(217,119,6,0.25)]",
          "font-mono",
        )}
      >
        <header
          className={cn(
            "flex",
            "items-center",
            "gap-3",
            "px-4",
            "py-2.5",
            "border-b",
            "border-copper-mid/50",
            "shrink-0",
          )}
        >
          <span className={cn("text-[11px]", "text-info-mid", "truncate")}>
            {hierarchy}
          </span>
          <button
            type="button"
            data-conversation-close
            onClick={onClose}
            aria-label="Close conversation"
            className={cn(
              "ml-auto",
              "px-2",
              "py-0.5",
              "rounded-sm",
              "text-copper-mid",
              "hover:text-copper-bright",
              "cursor-pointer",
              "text-sm",
              "leading-none",
            )}
          >
            {"✕"}
          </button>
        </header>
        <div
          className={cn(
            "flex-1",
            "overflow-auto",
            // column-reverse: the browser NATIVELY pins scrollTop=0
            // to the bottom — opens at the newest rows and stays
            // pinned through content growth with zero JS, and a user
            // scrolling up is off the pin until they return. The DOM
            // renders newest-first to match.
            "flex",
            "flex-col-reverse",
            "[overflow-anchor:none]",
            "px-4",
            "py-3",
            "gap-3",
            "text-[11px]",
            "text-[#c3bfbb]",
          )}
        >
          {/* Newest first (column-reverse flips it back visually). */}
          {[...blocks].reverse().map((block, i) => (
            <BlockCard key={blocks.length - 1 - i} block={block} />
          ))}
          {!live && (
            <div className={cn("self-center", "py-2")}>
              <LoadingDots marker="data-conversation-loading" />
            </div>
          )}
          {live && blocks.length === 0 && (
            <div
              className={cn("self-center", "py-2", "text-info-dim")}
              data-conversation-empty
            >
              no conversation yet
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

/** One conversation block: an indicator header (what it is, who from,
 * when) and its parts, each rendered per kind. */
function BlockCard({ block }: { block: ConversationBlock }) {
  const scope = blockScope(block);
  const at = blockTimestamp(block);
  return (
    <section
      data-conversation-block={block.type}
      className={cn(
        "flex",
        "flex-col",
        "gap-1",
        "rounded-sm",
        "border",
        "border-copper-mid/40",
        "px-3",
        "py-2",
      )}
    >
      <div className={cn("flex", "flex-row", "items-center", "gap-2")}>
        <KindChip label={scope} />
        {block.type === "client_notification" &&
          block.sender_agent_instance_hierarchy != null && (
            <span className={cn("text-[9px]", "text-info-mid", "truncate")}>
              from {block.sender_agent_instance_hierarchy}
            </span>
          )}
        {(block.type === "request_message_tool" ||
          block.type === "tool_response") && (
          <span className={cn("text-[9px]", "text-info-dim", "truncate")}>
            {block.tool_call_id}
          </span>
        )}
        {at !== null && (
          <span
            className={cn(
              "ml-auto",
              "text-[9px]",
              "text-info-dim",
              "tabular-nums",
              "whitespace-nowrap",
            )}
          >
            {formatAgo(at)}
          </span>
        )}
      </div>
      <BlockBody block={block} scope={scope} />
    </section>
  );
}

/** The block's content: every part in order, each per its kind. */
function BlockBody({
  block,
  scope,
}: {
  block: ConversationBlock;
  scope: string;
}) {
  switch (block.type) {
    case "vector_response_vote":
      return (
        <pre
          className={cn(
            "text-[9px]",
            "whitespace-pre-wrap",
            "break-words",
            "leading-snug",
          )}
        >
          {JSON.stringify(block.vote, null, 2)}
        </pre>
      );
    case "vector_request_choices":
      return (
        <div className={cn("flex", "flex-col", "gap-1.5")}>
          {block.choices.map((choice, i) => (
            <div key={i} className={cn("flex", "flex-col", "gap-1")}>
              <KindChip label={`choice · ${choice.key}`} />
              <PartList scope={scope} parts={choice.parts} />
            </div>
          ))}
        </div>
      );
    default:
      return <PartList scope={scope} parts={block.parts} />;
  }
}

/** A block's parts in order. Each part gets its body; a part whose
 * kind differs from plain scope-content (tool calls especially) gets
 * its own mini indicator above the body. */
function PartList({
  scope,
  parts,
}: {
  scope: string;
  parts: ConversationRow[];
}) {
  return (
    <div className={cn("flex", "flex-col", "gap-1.5")}>
      {parts.map((row, i) => {
        const { label, body } = describeRow(scope, row);
        if (body === null) return null;
        // Plain text parts read as the block's natural content; every
        // other kind announces itself.
        const announce = row.content.type !== "text";
        return (
          <div key={i} className={cn("flex", "flex-col", "gap-0.5")}>
            {announce && <KindChip label={label} />}
            <div className={cn("break-words", "select-text", "leading-4")}>
              {body}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/** The block's most recent timestamp — its last part's delivery, or
 * the notification's queue time as a fallback. */
function blockTimestamp(block: ConversationBlock): string | null {
  switch (block.type) {
    case "vector_response_vote":
      return null;
    case "vector_request_choices": {
      const lastChoice =
        block.choices.length > 0
          ? block.choices[block.choices.length - 1]
          : null;
      const parts = lastChoice?.parts ?? [];
      return parts.length > 0 ? parts[parts.length - 1].delivered_at : null;
    }
    case "client_notification": {
      const last =
        block.parts.length > 0
          ? block.parts[block.parts.length - 1].delivered_at
          : null;
      return last ?? block.queued_at ?? null;
    }
    default:
      return block.parts.length > 0
        ? block.parts[block.parts.length - 1].delivered_at
        : null;
  }
}
