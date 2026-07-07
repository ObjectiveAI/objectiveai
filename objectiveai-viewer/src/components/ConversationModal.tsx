import { Fragment, useEffect, useRef, useState } from "react";
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
          <ConversationBody logs={logs} />
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

/** A disclosure chevron: points right when closed, rotates down on
 * open. Inline-sized and inherits the toggle's text color. */
function Chevron({ open }: { open: boolean }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="13"
      height="13"
      fill="none"
      stroke="currentColor"
      strokeWidth={2.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={cn("shrink-0", "transition-transform", open && "rotate-90")}
    >
      <path d="m9 18 6-6-6-6" />
    </svg>
  );
}

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
    <div data-tool-section={name} className={cn("flex", "flex-col", "gap-0.5")}>
      <button
        type="button"
        data-tool-toggle
        onClick={() => setOpen((v) => !v)}
        className={cn(
          "self-start",
          "flex",
          "items-center",
          "gap-1.5",
          "text-info-dim",
          "hover:text-info-bright",
          "cursor-pointer",
        )}
      >
        <Chevron open={open} />
        <span className={cn("text-[9px]", "uppercase", "tracking-widest")}>
          tool
        </span>
        <span className={cn("text-info-mid")}>{name}</span>
      </button>
      {open && (
        <div
          className={cn(
            "ml-1.5",
            "pl-2",
            "border-l",
            "border-node-border",
            "flex",
            "flex-col",
            "gap-1.5",
          )}
        >
          {call !== undefined && (
            <div className={cn("flex", "flex-col", "gap-1")}>
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
            <div className={cn("flex", "flex-col", "gap-1")}>
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
        </div>
      )}
    </div>
  );
}

/** A log tool-response row wrapped as a [`ToolResponseSource`]. */
function logResponse(item: ToolResponseItem | undefined): ToolResponseSource {
  return item !== undefined ? { log: item } : null;
}

/** A coalesced conversation is a sequence of role RUNS: consecutive
 * blocks of the same role (user / assistant / notification) share
 * ONE badge, their content listed in order. Tool sections and
 * reasoning are assistant-role blocks (no badge of their own). */
type BlockRole = "user" | "assistant" | "notification";

interface Block {
  key: string;
  role: BlockRole;
  from?: string;
  at?: string;
  node: React.ReactNode;
}

interface Group {
  role: BlockRole;
  from?: string;
  at?: string;
  blocks: Block[];
}

/** The conversation body: fetch every request body up front (so the
 * ordered block list is fully known), flatten logs + request
 * messages into role-tagged blocks, coalesce consecutive same-role
 * runs, and render one badge per run — newest at the bottom. */
function ConversationBody({
  logs,
}: {
  logs: readonly CliCommandAgentsLogsListResponseItem[] | null;
}) {
  const requestBodies = useRequestBodies(logs);
  if (logs === null || requestBodies === null) {
    return <LoadingDots marker="data-conversation-loading" />;
  }
  if (logs.length === 0) {
    return <span className={cn("text-info-dim")}>no history</span>;
  }
  const groups = coalesce(flattenBlocks(logs, requestBodies));
  return (
    <>
      {groups
        .map((group, i) => <BlockGroup key={i} group={group} />)
        .reverse()}
    </>
  );
}

/** One role run: a single badge (role + the run's first block's
 * sender/date) over its blocks in order. */
function BlockGroup({ group }: { group: Group }) {
  return (
    <div
      data-block-group={group.role}
      className={cn("flex", "flex-col", "gap-1")}
    >
      <KindLabel
        note={group.from !== undefined ? `from ${group.from}` : undefined}
        at={group.at}
      >
        {group.role}
      </KindLabel>
      {group.blocks.map((b) => (
        <Fragment key={b.key}>{b.node}</Fragment>
      ))}
    </div>
  );
}

/** Group consecutive blocks of the same role — the coalescing rule:
 * the next badge is always a different role. */
function coalesce(blocks: Block[]): Group[] {
  const groups: Group[] = [];
  for (const b of blocks) {
    const last = groups[groups.length - 1];
    if (last !== undefined && last.role === b.role) {
      last.blocks.push(b);
    } else {
      groups.push({ role: b.role, from: b.from, at: b.at, blocks: [b] });
    }
  }
  return groups;
}

/** Flatten the ordered log rows (with requests expanded via their
 * fetched bodies) into role-tagged, badge-less blocks. tool_response
 * rows are consumed into their assistant's tool section; vector /
 * function requests are skipped. */
function flattenBlocks(
  logs: readonly CliCommandAgentsLogsListResponseItem[],
  requestBodies: ReadonlyMap<number, RequestMessage[]>,
): Block[] {
  const toolResponses = pairToolResponses(logs);
  const blocks: Block[] = [];
  logs.forEach((item, i) => {
    const base = `${item.type}-${i}`;
    switch (item.type) {
      case "assistant_response": {
        const at = item.parts[0]?.delivered_at;
        for (const part of item.parts) {
          const key = `${base}-${part.id}`;
          if (part.type === "reasoning") {
            blocks.push({
              key,
              role: "assistant",
              at,
              node: <ReasoningPart id={part.id} />,
            });
          } else if (part.type === "tool_call") {
            blocks.push({
              key,
              role: "assistant",
              at,
              node: (
                <ToolSection
                  name={part.function_name}
                  call={{ id: part.id }}
                  response={logResponse(toolResponses.get(part.tool_call_id))}
                />
              ),
            });
          } else {
            blocks.push({
              key,
              role: "assistant",
              at,
              node: (
                <>
                  {part.type === "refusal" && (
                    <span className={cn("text-info-dim")}>refusal</span>
                  )}
                  <LogPart id={part.id} />
                </>
              ),
            });
          }
        }
        break;
      }
      case "client_notification": {
        const from = item.sender_agent_instance_hierarchy;
        const at = item.queued_at;
        for (const part of item.parts) {
          blocks.push({
            key: `${base}-${part.id}`,
            role: "notification",
            from,
            at,
            node: <LogPart id={part.id} />,
          });
        }
        break;
      }
      case "agent_completion_request":
        pushRequestBlocks(
          blocks,
          base,
          requestBodies.get(item.id) ?? [],
          item.sender_agent_instance_hierarchy,
          item.delivered_at,
        );
        break;
      // tool_response rows are consumed above; vector / function
      // requests aren't part of an agent's message flow.
    }
  });
  return blocks;
}

/** Expand a request body's messages into blocks: user content;
 * assistant reasoning / content / tool-call sections / refusal; and
 * tool messages paired into their assistant's tool section (an
 * orphan tool message — its call in a prior log — renders as an
 * assistant-side response-only section). */
function pushRequestBlocks(
  blocks: Block[],
  base: string,
  messages: RequestMessage[],
  from: string,
  at: string,
): void {
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
  messages.forEach((m, j) => {
    const mk = `${base}-m${j}`;
    switch (m.role) {
      case "user":
        blocks.push({
          key: mk,
          role: "user",
          from,
          at,
          node: <InlineContent content={m.content} />,
        });
        break;
      case "assistant":
        if (m.reasoning != null && m.reasoning !== "") {
          blocks.push({
            key: `${mk}-r`,
            role: "assistant",
            from,
            at,
            node: <ReasoningPart text={m.reasoning} />,
          });
        }
        if (m.content != null) {
          blocks.push({
            key: `${mk}-c`,
            role: "assistant",
            from,
            at,
            node: <InlineContent content={m.content} />,
          });
        }
        for (const tc of m.tool_calls ?? []) {
          const answer = toolByCallId.get(tc.id);
          blocks.push({
            key: `${mk}-tc-${tc.id}`,
            role: "assistant",
            from,
            at,
            node: (
              <ToolSection
                name={tc.function.name}
                call={{ args: tc.function.arguments }}
                response={
                  answer !== undefined ? { inline: answer.content } : null
                }
              />
            ),
          });
        }
        if (m.refusal != null && m.refusal !== "") {
          blocks.push({
            key: `${mk}-refusal`,
            role: "assistant",
            from,
            at,
            node: (
              <>
                <span className={cn("text-info-dim")}>refusal</span>
                <InlineContent content={m.refusal} />
              </>
            ),
          });
        }
        break;
      case "tool":
        // Answered inside its assistant's section; only a tool
        // message whose call lives in a prior log shows standalone.
        if (calledIds.has(m.tool_call_id)) return;
        blocks.push({
          key: mk,
          role: "assistant",
          from,
          at,
          node: (
            <ToolSection name="tool response" response={{ inline: m.content }} />
          ),
        });
        break;
    }
  });
}

/** Fetch every `agent_completion_request` body referenced by `logs`
 * up front (requests are few — spawn boundaries), returning a map of
 * id → messages, or `null` while any are still loading. A request
 * that fails or isn't one resolves to no messages. */
function useRequestBodies(
  logs: readonly CliCommandAgentsLogsListResponseItem[] | null,
): ReadonlyMap<number, RequestMessage[]> | null {
  const [bodies, setBodies] = useState<Map<number, RequestMessage[]> | null>(
    null,
  );
  useEffect(() => {
    if (logs === null) {
      setBodies(null);
      return;
    }
    const ids = logs
      .filter((l) => l.type === "agent_completion_request")
      .map((l) => l.id);
    let cancelled = false;
    setBodies(null);
    void (async () => {
      const map = new Map<number, RequestMessage[]>();
      let executor: Awaited<ReturnType<typeof websocketExecutor>> | null = null;
      try {
        executor = await websocketExecutor();
      } catch {
        executor = null;
      }
      await Promise.all(
        ids.map(async (id) => {
          if (executor === null) {
            map.set(id, []);
            return;
          }
          try {
            const resp = await agentsLogsOpenExecute(executor, { id });
            map.set(
              id,
              resp.type === "agent_completion_request"
                ? resp.body.messages
                : [],
            );
          } catch {
            map.set(id, []);
          }
        }),
      );
      if (!cancelled) {
        setBodies(map);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [logs]);
  return bodies;
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
          "flex",
          "items-center",
          "gap-1.5",
          "text-info-dim",
          "hover:text-info-bright",
          "cursor-pointer",
        )}
      >
        <Chevron open={open} />
        <span className={cn("text-[9px]", "uppercase", "tracking-widest")}>
          reasoning
        </span>
      </button>
      {open && (
        <div
          className={cn(
            "ml-1.5",
            "pl-2",
            "border-l",
            "border-node-border",
            "flex",
            "flex-col",
            "gap-1",
          )}
        >
          {"id" in props ? (
            <LogPart id={props.id} />
          ) : (
            <InlineContent content={props.text} />
          )}
        </div>
      )}
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
