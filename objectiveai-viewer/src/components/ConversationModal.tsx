import { useEffect, useLayoutEffect, useRef, useState } from "react";
import cn from "classnames";
import {
  agentsLogsOpenExecute,
  type CliCommandAgentsLogsListResponseItem,
  type CliCommandAgentsLogsOpenResponse,
} from "@objectiveai/sdk";
import { useAgent } from "../hooks/useAgent";
import { websocketExecutor } from "../lib/websocket-executor";
import { LoadingDots } from "./LoadingDots";
import { Markdown } from "./Markdown";

/**
 * The per-agent conversation popup: near-fullscreen panel over a
 * dim backdrop, closable via the X, Escape, or a backdrop click.
 *
 * Renders the agent's HISTORICAL conversation — `useAgent().logs`,
 * the ordered `agents/logs/list` rows. The live streaming
 * completions (and the chat input) come later; the request rows
 * precede their completions in the list, so the log rows stand on
 * their own for now.
 *
 * No bulk content load: each ID-addressable part is its own
 * [`LogPart`], fetching its body via `agents/logs/open` only once
 * it scrolls into view — everything shows loading dots and resolves
 * one part at a time.
 */
export function ConversationModal({
  hierarchy,
  onClose,
}: {
  hierarchy: string;
  onClose: () => void;
}) {
  const { logs } = useAgent(hierarchy);
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const contentRef = useRef<HTMLDivElement | null>(null);
  // Bottom bias: pinned until the USER scrolls away from the bottom.
  const stickyRef = useRef(true);
  // True while a scroll we caused is in flight — those events must
  // not be mistaken for user intent.
  const snappingRef = useRef(false);

  const snapToBottom = () => {
    const el = bodyRef.current;
    if (el === null) return;
    snappingRef.current = true;
    el.scrollTop = el.scrollHeight;
    // The resulting scroll event lands in a later task.
    setTimeout(() => {
      snappingRef.current = false;
    }, 0);
  };

  // The conversation opens at the BOTTOM — the newest rows (which
  // also makes the lazy parts nearest the present load first).
  useLayoutEffect(() => {
    if (logs === null) return;
    snapToBottom();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [logs]);

  // Lazy parts resolving above/below grow the content and would
  // drift the viewport off the bottom — while pinned, every content
  // resize snaps back to the bottom.
  useLayoutEffect(() => {
    const el = bodyRef.current;
    const content = contentRef.current;
    if (el === null || content === null) return;
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (stickyRef.current) {
        snapToBottom();
      }
    });
    observer.observe(content);
    return () => {
      observer.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [logs]);

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
          ref={bodyRef}
          onScroll={() => {
            // Only USER scrolls may unpin — our own snaps are flagged.
            if (snappingRef.current) return;
            const el = bodyRef.current;
            if (el === null) return;
            stickyRef.current =
              el.scrollHeight - el.scrollTop - el.clientHeight < 40;
          }}
          className={cn(
            "flex-1",
            "overflow-auto",
            // The browser's own scroll anchoring also adjusts
            // scrollTop on content resizes, firing scroll events
            // indistinguishable from the user's and racing the pin —
            // we own the anchoring, so turn the native one off.
            "[overflow-anchor:none]",
            "px-4",
            "py-3",
            "text-[11px]",
            "text-[#c3bfbb]",
          )}
        >
          <div ref={contentRef} className={cn("flex", "flex-col", "gap-3")}>
            {logs === null ? (
              <LoadingDots marker="data-conversation-loading" />
            ) : logs.length === 0 ? (
              <span className={cn("text-info-dim")}>no history</span>
            ) : (
              logs.map((item, index) => (
                <LogRow
                  key={`${item.type}-${index}`}
                  item={item}
                  toolResponses={pairToolResponses(logs)}
                />
              ))
            )}
            {/* TODO: the live completions tail + the chat input land
                here in a later pass. */}
          </div>
        </div>
      </div>
    </div>
  );
}

/** The little copper kind label heading each block. */
function KindLabel({ children }: { children: string }) {
  return (
    <span
      className={cn(
        "self-start",
        "px-1.5",
        "py-px",
        "rounded-sm",
        "border",
        "border-copper-mid/70",
        "bg-copper-warm/10",
        "text-copper-bright",
      )}
    >
      {children}
    </span>
  );
}

/** A tool_response list row. */
type ToolResponseItem = Extract<
  CliCommandAgentsLogsListResponseItem,
  { type: "tool_response" }
>;

/** First tool_response per tool_call_id — the call/response pairing
 * the sections render. Orphan responses (no matching call) simply
 * never render. */
function pairToolResponses(
  logs: readonly CliCommandAgentsLogsListResponseItem[],
): ReadonlyMap<string, ToolResponseItem> {
  const map = new Map<string, ToolResponseItem>();
  for (const item of logs) {
    if (item.type === "tool_response" && !map.has(item.tool_call_id)) {
      map.set(item.tool_call_id, item);
    }
  }
  return map;
}

/** One tool exchange: the call's arguments and (when present) its
 * response, stacked as a request/response card headed by the tool's
 * NAME — the pairing is visual, no ids shown. */
function ToolSection({
  name,
  argsId,
  response,
}: {
  name: string;
  argsId: number;
  response: ToolResponseItem | null;
}) {
  const label = cn(
    "text-[9px]",
    "uppercase",
    "tracking-widest",
    "text-info-dim",
  );
  return (
    <div
      data-tool-section={name}
      className={cn(
        "rounded-sm",
        "border",
        "border-copper-mid/50",
        "overflow-hidden",
      )}
    >
      <div
        className={cn(
          "px-2.5",
          "py-1",
          "bg-copper-warm/10",
          "border-b",
          "border-copper-mid/50",
          "text-copper-bright",
        )}
      >
        {name}
      </div>
      <div className={cn("px-2.5", "py-1.5", "flex", "flex-col", "gap-1")}>
        <span className={label}>call</span>
        <LogPart id={argsId} json />
      </div>
      {response !== null && (
        <div
          className={cn(
            "px-2.5",
            "py-1.5",
            "border-t",
            "border-copper-mid/30",
            "flex",
            "flex-col",
            "gap-1",
          )}
        >
          <span className={label}>response</span>
          {response.parts.map((part) => (
            <LogPart key={part.id} id={part.id} />
          ))}
        </div>
      )}
    </div>
  );
}

/** One `agents/logs/list` row. Request rows are bare markers; the
 * part-bearing rows render one lazily-loaded [`LogPart`] each. Tool
 * calls render as paired call/response sections; bare tool_response
 * rows render nothing here (they live inside their call's section). */
function LogRow({
  item,
  toolResponses,
}: {
  item: CliCommandAgentsLogsListResponseItem;
  toolResponses: ReadonlyMap<string, ToolResponseItem>;
}) {
  switch (item.type) {
    case "agent_completion_request":
    case "vector_completion_request":
    case "function_execution_request":
      return (
        <div
          data-log-row={item.type}
          className={cn("text-info-dim", "flex", "items-center", "gap-1.5")}
        >
          <span>{"→"}</span>
          <span>{item.type.replace(/_/g, " ")}</span>
          <span className={cn("truncate")}>
            from {item.sender_agent_instance_hierarchy}
          </span>
        </div>
      );
    case "assistant_response":
      return (
        <div
          data-log-row={item.type}
          className={cn("flex", "flex-col", "gap-1")}
        >
          <KindLabel>assistant</KindLabel>
          {item.parts.map((part) =>
            part.type === "reasoning" ? (
              // Hidden by default — expanding also defers the fetch
              // until someone actually wants to read it.
              <ReasoningPart key={part.id} id={part.id} />
            ) : part.type === "tool_call" ? (
              // Call + its response, paired implicitly — no ids on
              // display.
              <ToolSection
                key={part.id}
                name={part.function_name}
                argsId={part.id}
                response={toolResponses.get(part.tool_call_id) ?? null}
              />
            ) : (
              <div key={part.id} className={cn("flex", "flex-col", "gap-0.5")}>
                {part.type === "refusal" && (
                  <span className={cn("text-info-dim")}>refusal</span>
                )}
                <LogPart id={part.id} />
              </div>
            ),
          )}
        </div>
      );
    case "tool_response":
      // Rendered inside its call's ToolSection; orphans (no matching
      // call) are ignored by design.
      return null;
    case "client_notification":
      return (
        <div
          data-log-row={item.type}
          className={cn("flex", "flex-col", "gap-1")}
        >
          <KindLabel>message</KindLabel>
          <span className={cn("text-info-dim", "truncate")}>
            from {item.sender_agent_instance_hierarchy}
          </span>
          {item.parts.map((part) => (
            <LogPart key={part.id} id={part.id} />
          ))}
        </div>
      );
  }
}

/** Pretty-print a JSON text body; unparsable input passes through
 * verbatim. */
function prettyJson(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

/** A reasoning part: collapsed to a disclosure line by default —
 * the body (and its fetch) only exists once opened. */
function ReasoningPart({ id }: { id: number }) {
  const [open, setOpen] = useState(false);
  return (
    <div className={cn("flex", "flex-col", "gap-0.5")}>
      <button
        type="button"
        data-reasoning-toggle
        onClick={() => setOpen((v) => !v)}
        className={cn(
          "self-start",
          "text-info-dim",
          "hover:text-info-bright",
          "cursor-pointer",
        )}
      >
        {open ? "\u25be reasoning" : "\u25b8 reasoning"}
      </button>
      {open && <LogPart id={id} />}
    </div>
  );
}

/**
 * One ID-addressable log part. Fetches its body via
 * `agents/logs/open` the first time it becomes visible (immediately
 * where IntersectionObserver doesn't exist, e.g. jsdom) — loading
 * dots until then.
 */
function LogPart({ id, json = false }: { id: number; json?: boolean }) {
  const [content, setContent] =
    useState<CliCommandAgentsLogsOpenResponse | null>(null);
  const [failed, setFailed] = useState(false);
  const ref = useRef<HTMLDivElement | null>(null);
  const startedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    const start = () => {
      if (startedRef.current) return;
      startedRef.current = true;
      void (async () => {
        try {
          const executor = await websocketExecutor();
          const response = await agentsLogsOpenExecute(executor, { id });
          if (cancelled) return;
          if (response.type === "error") {
            setFailed(true);
            return;
          }
          setContent(response);
        } catch {
          if (!cancelled) {
            setFailed(true);
          }
        }
      })();
    };
    if (typeof IntersectionObserver === "undefined") {
      start();
      return () => {
        cancelled = true;
      };
    }
    const el = ref.current;
    if (el === null) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        start();
        observer.disconnect();
      }
    });
    observer.observe(el);
    return () => {
      cancelled = true;
      observer.disconnect();
    };
  }, [id]);

  return (
    <div ref={ref} data-log-part={id}>
      {failed ? (
        <span className={cn("text-error")}>failed to load</span>
      ) : content === null ? (
        <LoadingDots marker="data-part-loading" />
      ) : (
        <PartContent content={content} json={json} />
      )}
    </div>
  );
}

/** The resolved body, by kind. Text is the star; media renders as a
 * minimal preview/label for now. `json` text bodies (tool-call
 * arguments) pretty-print instead of rendering as markdown. */
function PartContent({
  content,
  json = false,
}: {
  content: CliCommandAgentsLogsOpenResponse;
  json?: boolean;
}) {
  switch (content.type) {
    case "text":
      if (json) {
        return (
          <pre
            data-json-part
            className={cn(
              "text-[10px]",
              "text-left",
              "whitespace-pre-wrap",
              "break-words",
              "leading-snug",
            )}
          >
            {prettyJson(content.text)}
          </pre>
        );
      }
      return <Markdown>{content.text}</Markdown>;
    case "image":
      return (
        <img
          src={content.url}
          alt="agent image"
          className={cn("max-h-64", "rounded-sm")}
        />
      );
    case "video":
      return (
        <span className={cn("text-info-dim")}>video: {content.url}</span>
      );
    case "audio":
      return (
        <span className={cn("text-info-dim")}>audio ({content.format})</span>
      );
    case "file":
      return <span className={cn("text-info-dim")}>file</span>;
    default:
      // Request blobs never resolve from part ids; render a marker
      // if one ever does.
      return <span className={cn("text-info-dim")}>{content.type}</span>;
  }
}
