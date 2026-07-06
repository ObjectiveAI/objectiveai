import { useEffect, useRef, useState } from "react";
import cn from "classnames";
import {
  agentsLogsOpenExecute,
  type AgentCompletionsMessageMessage,
  type AgentCompletionsMessageRichContent,
  type AgentCompletionsMessageRichContentPart,
  type CliCommandAgentsLogsListResponseItem,
  type CliCommandAgentsLogsOpenResponse,
} from "@objectiveai/sdk";
import { useAgent } from "../hooks/useAgent";
import { formatAgo } from "../lib/formatAgo";
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
          {/* TODO: the live completions tail + the chat input land
              here in a later pass. */}
          {logs === null ? (
            <LoadingDots marker="data-conversation-loading" />
          ) : logs.length === 0 ? (
            <span className={cn("text-info-dim")}>no history</span>
          ) : (
            logs
              .map((item, index) => (
                <LogRow
                  key={`${item.type}-${index}`}
                  item={item}
                  toolResponses={pairToolResponses(logs)}
                />
              ))
              .reverse()
          )}
        </div>
      </div>
    </div>
  );
}

/** The little copper kind label heading each block: the badge, an
 * optional inline note directly to its right, and an optional
 * relative timestamp pushed to the far right. */
function KindLabel({
  children,
  note,
  at,
}: {
  children: string;
  note?: string;
  at?: string;
}) {
  return (
    <div className={cn("flex", "items-center", "gap-1.5", "self-stretch")}>
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
        {children}
      </span>
      {note !== undefined && (
        <span className={cn("text-info-dim", "truncate")}>{note}</span>
      )}
      {at !== undefined && (
        <span
          data-badge-ago
          className={cn(
            "ml-auto",
            "text-[9px]",
            "text-info-dim",
            "tabular-nums",
          )}
        >
          {formatAgo(at)}
        </span>
      )}
    </div>
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

/** A tool call's arguments source: a log id to fetch, or the inline
 * JSON string from an assistant message. */
type ToolCallSource = { id: number } | { args: string };
/** A tool response's source: a log row (parts fetch by id), inline
 * content (a tool message), or none (unanswered call). */
type ToolResponseSource =
  | { log: ToolResponseItem }
  | { inline: AgentCompletionsMessageRichContent }
  | null;

/** One tool exchange: the call's arguments and (when present) its
 * response, stacked as a request/response card headed by the tool's
 * NAME — the pairing is visual, no ids shown. Collapsed by default;
 * `call` omitted renders a response-only card (an orphan tool
 * response whose call lives in a prior log). */
function ToolSection({
  name,
  call,
  response,
}: {
  name: string;
  call?: ToolCallSource;
  response: ToolResponseSource;
}) {
  // Closed by default — opening also defers the log-source fetches
  // until someone actually wants to look.
  const [open, setOpen] = useState(false);
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
      <button
        type="button"
        data-tool-toggle
        onClick={() => setOpen((v) => !v)}
        className={cn(
          "w-full",
          "flex",
          "items-center",
          "gap-1.5",
          "px-2.5",
          "py-1",
          "bg-copper-warm/10",
          "text-copper-bright",
          "text-left",
          "cursor-pointer",
          open && cn("border-b", "border-copper-mid/50"),
        )}
      >
        <span className={cn("text-info-dim")}>
          {open ? "\u25be" : "\u25b8"}
        </span>
        {name}
      </button>
      {open && (
        <>
          {call !== undefined && (
            <div
              className={cn("px-2.5", "py-1.5", "flex", "flex-col", "gap-1")}
            >
              <span className={label}>call</span>
              {"args" in call ? (
                <MediaView
                  media={{ kind: "text", text: call.args, json: true }}
                />
              ) : (
                <LogPart id={call.id} json />
              )}
            </div>
          )}
          {response !== null && (
            <div
              className={cn(
                "px-2.5",
                "py-1.5",
                call !== undefined && cn("border-t", "border-copper-mid/30"),
                "flex",
                "flex-col",
                "gap-1",
              )}
            >
              <span className={label}>response</span>
              {"log" in response ? (
                response.log.parts.map((part) => (
                  <LogPart key={part.id} id={part.id} />
                ))
              ) : (
                <InlineContent content={response.inline} />
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}

/** A log tool-response row wrapped as a [`ToolResponseSource`]. */
function logResponse(item: ToolResponseItem | undefined): ToolResponseSource {
  return item !== undefined ? { log: item } : null;
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
      return (
        <RequestSection
          id={item.id}
          sender={item.sender_agent_instance_hierarchy}
          at={item.delivered_at}
        />
      );
    case "vector_completion_request":
    case "function_execution_request":
      return (
        <div
          data-log-row={item.type}
          className={cn("flex", "flex-col", "gap-1")}
        >
          <KindLabel
            note={`from ${item.sender_agent_instance_hierarchy}`}
            at={item.delivered_at}
          >
            {item.type === "vector_completion_request"
              ? "vector request"
              : "function request"}
          </KindLabel>
        </div>
      );
    case "assistant_response":
      return (
        <div
          data-log-row={item.type}
          className={cn("flex", "flex-col", "gap-1")}
        >
          <KindLabel at={item.parts[0]?.delivered_at}>assistant</KindLabel>
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
                call={{ id: part.id }}
                response={logResponse(toolResponses.get(part.tool_call_id))}
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
          <KindLabel
            note={`from ${item.sender_agent_instance_hierarchy}`}
            at={item.queued_at}
          >
            notification
          </KindLabel>
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
 * the body (and, for the log source, its fetch) only exists once
 * opened. Source is a log id (fetch) or inline reasoning text. */
function ReasoningPart(props: { id: number } | { text: string }) {
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
      {open &&
        ("id" in props ? (
          <LogPart id={props.id} />
        ) : (
          <InlineContent content={props.text} />
        ))}
    </div>
  );
}

/**
 * Lazily open one log id via `agents/logs/open`, fetching the first
 * time the returned `ref` element becomes visible (immediately where
 * IntersectionObserver doesn't exist, e.g. jsdom). Shared by
 * [`LogPart`] and [`RequestSection`].
 */
function useLazyOpen(id: number): {
  content: CliCommandAgentsLogsOpenResponse | null;
  failed: boolean;
  ref: React.RefObject<HTMLDivElement | null>;
} {
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

  return { content, failed, ref };
}

/** One ID-addressable log part: fetched on view, rendered through
 * the shared [`MediaView`]. */
function LogPart({ id, json = false }: { id: number; json?: boolean }) {
  const { content, failed, ref } = useLazyOpen(id);
  return (
    <div ref={ref} data-log-part={id}>
      {failed ? (
        <span className={cn("text-error")}>failed to load</span>
      ) : content === null ? (
        <LoadingDots marker="data-part-loading" />
      ) : (
        <MediaView media={openResponseToMedia(content, json)} />
      )}
    </div>
  );
}

/** Render inline `RichContent` (no fetch): a bare string as markdown
 * (or pretty JSON when `json`), or each part through the shared
 * [`MediaView`]. */
function InlineContent({
  content,
  json = false,
}: {
  content: AgentCompletionsMessageRichContent;
  json?: boolean;
}) {
  if (typeof content === "string") {
    return <MediaView media={{ kind: "text", text: content, json }} />;
  }
  return (
    <>
      {content.map((part, i) => (
        <MediaView key={i} media={richPartToMedia(part)} />
      ))}
    </>
  );
}

/** The normalized media descriptor both the fetched
 * `agents/logs/open` bodies and inline `RichContentPart`s map to, so
 * one renderer ([`MediaView`]) serves both sources. */
type Media =
  | { kind: "text"; text: string; json?: boolean }
  | { kind: "image"; url: string }
  | { kind: "audio"; data: string; format: string }
  | { kind: "video"; url: string }
  | {
      kind: "file";
      file_data?: string | null;
      file_id?: string | null;
      file_url?: string | null;
      filename?: string | null;
    };

/** An `agents/logs/open` response → [`Media`]. */
function openResponseToMedia(
  content: CliCommandAgentsLogsOpenResponse,
  json: boolean,
): Media {
  switch (content.type) {
    case "text":
      return { kind: "text", text: content.text, json };
    case "image":
      return { kind: "image", url: content.url };
    case "audio":
      return { kind: "audio", data: content.data, format: content.format };
    case "video":
      return { kind: "video", url: content.url };
    case "file":
      return {
        kind: "file",
        file_data: content.file_data,
        file_id: content.file_id,
        file_url: content.file_url,
        filename: content.filename,
      };
    default:
      // Request blobs never resolve from part ids.
      return { kind: "text", text: content.type };
  }
}

/** An inline `RichContentPart` → [`Media`]. */
function richPartToMedia(part: AgentCompletionsMessageRichContentPart): Media {
  switch (part.type) {
    case "text":
      return { kind: "text", text: part.text };
    case "image_url":
      return { kind: "image", url: part.image_url.url };
    case "input_audio":
      return {
        kind: "audio",
        data: part.input_audio.data,
        format: part.input_audio.format,
      };
    case "video_url":
    case "input_video":
      return { kind: "video", url: part.video_url.url };
    case "file":
      return {
        kind: "file",
        file_data: part.file.file_data,
        file_id: part.file.file_id,
        file_url: part.file.file_url,
        filename: part.file.filename,
      };
  }
}

/** Render one normalized media descriptor. Text is the star (markdown
 * or, for `json` tool-call arguments, pretty-printed); image / video
 * / audio get real players; file a download link. */
function MediaView({ media }: { media: Media }) {
  switch (media.kind) {
    case "text":
      if (media.json) {
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
            {prettyJson(media.text)}
          </pre>
        );
      }
      return <Markdown>{media.text}</Markdown>;
    case "image":
      return <MediaImage url={media.url} />;
    case "video":
      return (
        <video
          data-media-video
          src={media.url}
          controls
          preload="metadata"
          className={cn("max-w-full", "max-h-80", "rounded-sm")}
        />
      );
    case "audio":
      return (
        <audio
          data-media-audio
          src={`data:audio/${mediaMime(media.format)};base64,${media.data}`}
          controls
          preload="metadata"
          className={cn("max-w-full")}
        />
      );
    case "file":
      return <MediaFile file={media} />;
  }
}

/** Format → media MIME subtype for audio data URLs. `mp3` → `mpeg`,
 * `m4a` → `mp4`; everything else passes through (`wav`, `ogg`,
 * `webm`, `flac`, …). */
function mediaMime(format: string): string {
  const f = format.toLowerCase();
  if (f === "mp3") return "mpeg";
  if (f === "m4a") return "mp4";
  return f;
}

/** An image with a size cap and a load-failure fallback, so a bad or
 * blocked URL shows a line instead of nothing. */
function MediaImage({ url }: { url: string }) {
  const [failed, setFailed] = useState(false);
  if (failed) {
    return (
      <span data-media-image-failed className={cn("text-info-dim")}>
        image failed to load —{" "}
        <span className={cn("underline", "break-all")}>{url}</span>
      </span>
    );
  }
  return (
    <img
      data-media-image
      src={url}
      alt="agent image"
      loading="lazy"
      onError={() => setFailed(true)}
      className={cn(
        "max-w-full",
        "max-h-80",
        "object-contain",
        "rounded-sm",
      )}
    />
  );
}

/** A file attachment: a download link when data/url is present,
 * otherwise a dim label. Data-URL downloads never navigate the
 * webview. */
function MediaFile({
  file,
}: {
  file: {
    file_data?: string | null;
    file_id?: string | null;
    file_url?: string | null;
    filename?: string | null;
  };
}) {
  const name = file.filename ?? file.file_id ?? "attachment";
  const href =
    file.file_url != null
      ? file.file_url
      : file.file_data != null
        ? `data:application/octet-stream;base64,${file.file_data}`
        : null;
  if (href === null) {
    return <span className={cn("text-info-dim")}>file: {name}</span>;
  }
  return (
    <a
      data-media-file
      href={href}
      download={file.filename ?? "file"}
      className={cn("underline", "text-copper-bright", "cursor-pointer")}
    >
      {name}
    </a>
  );
}

/** A user / assistant / tool message from a request body. */
type RequestMessage = AgentCompletionsMessageMessage;
type AssistantMsg = Extract<RequestMessage, { role: "assistant" }>;
type ToolMsg = Extract<RequestMessage, { role: "tool" }>;

/** The unwrapped `agent_completion_request`: a dated `request`
 * header, then its body's messages — the only place the caller's
 * user input appears.
 *
 * Continuation / queue-delivery requests carry their content on the
 * continuation STATE, not `body.messages`, so those bodies are empty
 * and their user input is the nearby `client_notification` rows. An
 * empty request is treated as NON-EXISTENT — nothing renders (not
 * even a boundary) so there's no gap. Because emptiness is only
 * knowable after the fetch, the body is fetched eagerly on mount
 * (requests are few — spawn boundaries) and `null` renders until
 * then. */
function RequestSection({
  id,
  sender,
  at,
}: {
  id: number;
  sender: string;
  at: string;
}) {
  const [messages, setMessages] = useState<RequestMessage[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const executor = await websocketExecutor();
        const resp = await agentsLogsOpenExecute(executor, { id });
        if (cancelled) return;
        setMessages(
          resp.type === "agent_completion_request" ? resp.body.messages : [],
        );
      } catch {
        if (!cancelled) setMessages([]);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [id]);

  // Nothing at all while loading OR when empty — the request simply
  // does not exist in the conversation.
  if (messages === null || messages.length === 0) return null;

  // Flattened: no request wrapper. Each message renders as a
  // top-level row carrying the request's sender + date on its own
  // badge.
  return <RequestMessages messages={messages} sender={sender} at={at} />;
}

/** Render a request body's messages: user (content), assistant (the
 * shared assistant body), and tool responses paired into their
 * assistant's tool sections. A tool message whose call is NOT in
 * these messages (it lives in a prior log) renders as a standalone
 * response-only tool section. */
function RequestMessages({
  messages,
  sender,
  at,
}: {
  messages: RequestMessage[];
  sender: string;
  at: string;
}) {
  // The response per tool_call_id, and the set of ids an assistant
  // in these messages actually calls (so orphan tool messages —
  // answering a log-side call — still render).
  const toolByCallId = new Map<string, ToolMsg>();
  const calledIds = new Set<string>();
  for (const m of messages) {
    if (m.role === "tool" && !toolByCallId.has(m.tool_call_id)) {
      toolByCallId.set(m.tool_call_id, m);
    }
    if (m.role === "assistant") {
      for (const tc of m.tool_calls ?? []) {
        calledIds.add(tc.id);
      }
    }
  }
  return (
    <>
      {messages.map((m, i) => {
        switch (m.role) {
          case "user":
            return (
              <div
                key={i}
                data-request-message="user"
                className={cn("flex", "flex-col", "gap-1")}
              >
                <KindLabel note={`from ${sender}`} at={at}>
                  user
                </KindLabel>
                <InlineContent content={m.content} />
              </div>
            );
          case "assistant":
            return (
              <AssistantMessage
                key={i}
                msg={m}
                toolByCallId={toolByCallId}
                sender={sender}
                at={at}
              />
            );
          case "tool":
            // Answered inside its assistant's section above; only a
            // tool message whose call lives in a prior log shows
            // standalone.
            if (calledIds.has(m.tool_call_id)) return null;
            return (
              <ToolSection
                key={i}
                name="tool response"
                response={{ inline: m.content }}
              />
            );
        }
      })}
    </>
  );
}

/** An assistant message rendered with the SAME pieces as a log
 * assistant turn: collapsed reasoning, content, and one tool section
 * per tool call (paired with its inline tool response), then a
 * refusal. */
function AssistantMessage({
  msg,
  toolByCallId,
  sender,
  at,
}: {
  msg: AssistantMsg;
  toolByCallId: ReadonlyMap<string, ToolMsg>;
  sender: string;
  at: string;
}) {
  return (
    <div
      data-request-message="assistant"
      className={cn("flex", "flex-col", "gap-1")}
    >
      <KindLabel note={`from ${sender}`} at={at}>
        assistant
      </KindLabel>
      {msg.reasoning != null && msg.reasoning !== "" && (
        <ReasoningPart text={msg.reasoning} />
      )}
      {msg.content != null && <InlineContent content={msg.content} />}
      {(msg.tool_calls ?? []).map((tc) => {
        const answer = toolByCallId.get(tc.id);
        return (
          <ToolSection
            key={tc.id}
            name={tc.function.name}
            call={{ args: tc.function.arguments }}
            response={answer !== undefined ? { inline: answer.content } : null}
          />
        );
      })}
      {msg.refusal != null && msg.refusal !== "" && (
        <div className={cn("flex", "flex-col", "gap-0.5")}>
          <span className={cn("text-info-dim")}>refusal</span>
          <InlineContent content={msg.refusal} />
        </div>
      )}
    </div>
  );
}
