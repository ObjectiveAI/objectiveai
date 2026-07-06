import { useEffect, useRef, useState } from "react";
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
        "fixed",
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
            "px-4",
            "py-3",
            "flex",
            "flex-col",
            "gap-3",
            "text-[11px]",
            "text-[#c3bfbb]",
          )}
        >
          {logs === null ? (
            <LoadingDots marker="data-conversation-loading" />
          ) : logs.length === 0 ? (
            <span className={cn("text-info-dim")}>no history</span>
          ) : (
            logs.map((item, index) => (
              <LogRow key={`${item.type}-${index}`} item={item} />
            ))
          )}
          {/* TODO: the live completions tail + the chat input land
              here in a later pass. */}
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

/** One `agents/logs/list` row. Request rows are bare markers; the
 * part-bearing rows render one lazily-loaded [`LogPart`] each. */
function LogRow({ item }: { item: CliCommandAgentsLogsListResponseItem }) {
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
          {item.parts.map((part) => (
            <div key={part.id} className={cn("flex", "flex-col", "gap-0.5")}>
              {part.type === "tool_call" && (
                <span className={cn("text-info-dim")}>
                  tool call {part.function_name} ({part.tool_call_id})
                </span>
              )}
              {part.type === "reasoning" && (
                <span className={cn("text-info-dim")}>reasoning</span>
              )}
              {part.type === "refusal" && (
                <span className={cn("text-info-dim")}>refusal</span>
              )}
              <LogPart id={part.id} />
            </div>
          ))}
        </div>
      );
    case "tool_response":
      return (
        <div
          data-log-row={item.type}
          className={cn("flex", "flex-col", "gap-1")}
        >
          <KindLabel>tool</KindLabel>
          <span className={cn("text-info-dim")}>{item.tool_call_id}</span>
          {item.parts.map((part) => (
            <LogPart key={part.id} id={part.id} />
          ))}
        </div>
      );
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

/**
 * One ID-addressable log part. Fetches its body via
 * `agents/logs/open` the first time it becomes visible (immediately
 * where IntersectionObserver doesn't exist, e.g. jsdom) — loading
 * dots until then.
 */
function LogPart({ id }: { id: number }) {
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
        <PartContent content={content} />
      )}
    </div>
  );
}

/** The resolved body, by kind. Text is the star; media renders as a
 * minimal preview/label for now. */
function PartContent({
  content,
}: {
  content: CliCommandAgentsLogsOpenResponse;
}) {
  switch (content.type) {
    case "text":
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
