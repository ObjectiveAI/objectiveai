import { Fragment, useState } from "react";
import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import {
  useAgentInstance,
  type ConversationBlock,
} from "../hooks/useAgentInstance";
import type { AssistantPart, PartContent } from "./conversationContent";
import { useAgo } from "../hooks/useAgo";
import { LoadingDots } from "./LoadingDots";
import { Markdown } from "./Markdown";

/**
 * The per-agent conversation popup: near-fullscreen panel over a dim
 * backdrop, closable via the X, Escape, or a backdrop click.
 *
 * Renders the agent's WHOLE conversation from its own
 * `/agents/instances/{*aih}` connection ([`useAgentInstance`]) — the
 * blocks are the `agents logs list` `ResponseItem` mirror with content
 * INLINE, so no per-part fetches exist. The presentation is role-run
 * based: consecutive blocks of the same role (user / assistant /
 * notification / error) share ONE [`KindLabel`] badge; reasoning and
 * tool exchanges are assistant-side DISCLOSURES, collapsed by default
 * — a tool call pairs with its response block (by `tool_call_id`)
 * inside one [`ToolSection`], call arguments as pretty JSON. Media
 * renders for real: images, audio, video, file downloads.
 */
/**
 * One agent's WHOLE conversation, fullscreen-fill — the content of the
 * old popup as a standalone view (the agent conversation WINDOW's
 * body; no tabs, no footer). Owns its own `/agents/instances/{*aih}`
 * connection via [`useAgentInstance`]: blocks are the `agents logs
 * list` `ResponseItem` mirror with content INLINE. Role-run
 * presentation: consecutive same-role blocks share ONE [`KindLabel`]
 * badge; reasoning and tool exchanges are collapsed-by-default
 * DISCLOSURES — a tool call pairs with its response block by
 * `tool_call_id` inside one [`ToolSection`]. Media renders for real.
 */
export function ConversationView({
  transport,
  hierarchy,
}: {
  transport: ViewerTransport | null;
  hierarchy: string;
}) {
  const { blocks, live } = useAgentInstance(transport, hierarchy);
  return (
    <div
      data-conversation-view
      className={cn(
        "h-full",
        "overflow-auto",
        // column-reverse: the browser NATIVELY pins scrollTop=0 to the
        // bottom — opens at the newest rows and stays pinned through
        // content growth with zero JS, and a user scrolling up is off
        // the pin until they return. The DOM renders newest-first to
        // match.
        "flex",
        "flex-col-reverse",
        "[overflow-anchor:none]",
        "px-4",
        "py-3",
        "gap-3",
        "font-mono",
        "text-sm",
        "text-[#c3bfbb]",
      )}
    >
      <ConversationBody blocks={blocks} live={live} />
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
  // Live-updating relative date; "" (no `at`) renders nothing and
  // schedules nothing.
  const ago = useAgo(at ?? "");
  return (
    <div className={cn("flex", "items-center", "gap-1.5", "self-stretch")}>
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
        {children}
      </span>
      {note !== undefined && (
        <span className={cn("text-xs", "text-info-dim", "truncate")}>
          {note}
        </span>
      )}
      {at !== undefined && (
        <span
          data-badge-ago
          className={cn(
            "ml-auto",
            "text-xs",
            "text-info-dim",
            "tabular-nums",
          )}
        >
          {ago}
        </span>
      )}
    </div>
  );
}

/** A tool block — the response side of a tool exchange. */
type ToolBlock = Extract<
  ConversationBlock,
  { type: "tool_response" } | { type: "request_message_tool" }
>;

/** First tool block per tool_call_id — the call/response pairing the
 * sections render. */
function pairToolResponses(
  blocks: readonly ConversationBlock[],
): ReadonlyMap<string, ToolBlock> {
  const map = new Map<string, ToolBlock>();
  for (const block of blocks) {
    if (
      (block.type === "tool_response" ||
        block.type === "request_message_tool") &&
      !map.has(block.tool_call_id)
    ) {
      map.set(block.tool_call_id, block);
    }
  }
  return map;
}

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

/** A generic disclosure section in the tool/reasoning style: chevron
 * toggle + uppercase kind label (+ optional name), body indented
 * behind a left border, CLOSED by default. */
function Disclosure({
  kind,
  name,
  marker,
  children,
}: {
  kind: string;
  name?: string;
  marker?: Record<string, unknown>;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);
  return (
    <div {...(marker ?? {})} className={cn("flex", "flex-col", "gap-0.5")}>
      <button
        type="button"
        data-disclosure-toggle={kind}
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
          {kind}
        </span>
        {name !== undefined && (
          <span className={cn("text-info-mid")}>{name}</span>
        )}
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
          {children}
        </div>
      )}
    </div>
  );
}

/** One tool exchange: the call's arguments and (when present) its
 * response, stacked as a request/response card headed by the tool's
 * NAME — the pairing is visual, no ids shown. Collapsed by default;
 * `args` omitted renders a response-only card (an orphan tool
 * response whose call this stream never saw). */
function ToolSection({
  name,
  args,
  response,
}: {
  name: string;
  args?: string;
  response: ToolBlock | null;
}) {
  const label = cn(
    "text-[9px]",
    "uppercase",
    "tracking-widest",
    "text-info-dim",
  );
  return (
    <Disclosure kind="tool" name={name} marker={{ "data-tool-section": name }}>
      {args !== undefined && (
        <div className={cn("flex", "flex-col", "gap-1")}>
          <span className={label}>call</span>
          <MediaView media={{ kind: "text", text: args, json: true }} />
        </div>
      )}
      {response !== null && (
        <div className={cn("flex", "flex-col", "gap-1")}>
          <span className={label}>response</span>
          {response.parts.map((part, i) => (
            <MediaView key={i} media={contentToMedia(part.content)} />
          ))}
        </div>
      )}
    </Disclosure>
  );
}

/** A reasoning part: collapsed to a disclosure line by default. */
function ReasoningPart({ text }: { text: string }) {
  return (
    <Disclosure kind="reasoning" marker={{ "data-reasoning-part": "" }}>
      <MediaView media={{ kind: "text", text }} />
    </Disclosure>
  );
}

/** A coalesced conversation is a sequence of role RUNS: consecutive
 * blocks of the same role (user / assistant / notification / error)
 * share ONE badge, their content listed in order. Tool sections and
 * reasoning are assistant-role blocks (no badge of their own). */
type BlockRole = "user" | "assistant" | "notification" | "error";

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

/** The conversation body: flatten the stream's blocks into
 * role-tagged blocks, coalesce consecutive same-role runs, and
 * render one badge per run — newest at the bottom. */
function ConversationBody({
  blocks,
  live,
}: {
  blocks: readonly ConversationBlock[];
  live: boolean;
}) {
  if (!live && blocks.length === 0) {
    return <LoadingDots marker="data-conversation-loading" />;
  }
  if (blocks.length === 0) {
    return <span className={cn("text-info-dim")}>no history</span>;
  }
  const groups = coalesce(flattenBlocks(blocks));
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

/** Flatten the mirror blocks into role-tagged, badge-less display
 * blocks. Tool blocks are consumed into their call's [`ToolSection`]
 * (paired by `tool_call_id`); an orphan tool block — its call outside
 * this stream — renders as a response-only section in place.
 * Request/response assistant shapes are NOT distinguished. */
function flattenBlocks(source: readonly ConversationBlock[]): Block[] {
  const toolResponses = pairToolResponses(source);
  const consumed = new Set<ToolBlock>();
  // First pass: mark every tool block that a tool CALL will consume,
  // so an in-place orphan check below stays order-independent.
  for (const item of source) {
    if (
      item.type === "assistant_response" ||
      item.type === "request_message_assistant"
    ) {
      for (const part of item.parts) {
        if (part.type === "tool_call") {
          const answer = toolResponses.get(part.tool_call_id);
          if (answer !== undefined) consumed.add(answer);
        }
      }
    }
  }
  const blocks: Block[] = [];
  source.forEach((item, i) => {
    const base = `${item.type}-${i}`;
    switch (item.type) {
      case "assistant_response":
      case "request_message_assistant": {
        const at = item.parts[0]?.delivered_at;
        item.parts.forEach((part, j) => {
          const key = `${base}-${j}`;
          blocks.push({
            key,
            role: "assistant",
            at,
            node: assistantPartNode(part, toolResponses),
          });
        });
        break;
      }
      case "request_message_user": {
        const at = item.parts[0]?.delivered_at;
        item.parts.forEach((part, j) => {
          blocks.push({
            key: `${base}-${j}`,
            role: "user",
            at,
            node: <MediaView media={contentToMedia(part.content)} />,
          });
        });
        break;
      }
      case "client_notification": {
        const from = item.sender_agent_instance_hierarchy;
        const at = item.queued_at;
        item.parts.forEach((part, j) => {
          blocks.push({
            key: `${base}-${j}`,
            role: "notification",
            from,
            at,
            node: <MediaView media={contentToMedia(part.content)} />,
          });
        });
        break;
      }
      case "tool_response":
      case "request_message_tool": {
        // Answered inside its call's section; only an orphan (its
        // call outside this stream) shows standalone, response-only.
        if (consumed.has(item)) break;
        blocks.push({
          key: base,
          role: "assistant",
          at: item.parts[0]?.delivered_at,
          node: <ToolSection name="tool response" response={item} />,
        });
        break;
      }
      case "vector_request_choices": {
        // The choices a vector task voted over — request data, shown
        // as a user-side disclosure (closed by default).
        blocks.push({
          key: base,
          role: "user",
          at: item.choices[0]?.parts[0]?.delivered_at,
          node: (
            <Disclosure kind="choices" marker={{ "data-choices-section": "" }}>
              {item.choices.map((choice, j) => (
                <div key={j} className={cn("flex", "flex-col", "gap-1")}>
                  <span
                    className={cn(
                      "text-[9px]",
                      "uppercase",
                      "tracking-widest",
                      "text-info-dim",
                    )}
                  >
                    {choice.key}
                  </span>
                  {choice.parts.map((part, k) => (
                    <MediaView key={k} media={contentToMedia(part.content)} />
                  ))}
                </div>
              ))}
            </Disclosure>
          ),
        });
        break;
      }
      case "vector_response_vote": {
        // The agent's own vote — assistant-side disclosure.
        blocks.push({
          key: base,
          role: "assistant",
          node: (
            <Disclosure kind="vote" marker={{ "data-vote-section": "" }}>
              <MediaView
                media={{
                  kind: "text",
                  text: JSON.stringify(item.vote),
                  json: true,
                }}
              />
            </Disclosure>
          ),
        });
        break;
      }
      case "error": {
        // A logged failure — its own role run, value as pretty JSON.
        blocks.push({
          key: base,
          role: "error",
          at: item.delivered_at,
          node: (
            <MediaView
              media={{
                kind: "text",
                text:
                  typeof item.error === "string"
                    ? item.error
                    : JSON.stringify(item.error),
                json: typeof item.error !== "string",
              }}
            />
          ),
        });
        break;
      }
    }
  });
  return blocks;
}

/** One assistant-family part as its display node. */
function assistantPartNode(
  part: AssistantPart,
  toolResponses: ReadonlyMap<string, ToolBlock>,
): React.ReactNode {
  switch (part.type) {
    case "reasoning":
      return <ReasoningPart text={part.text} />;
    case "tool_call":
      return (
        <ToolSection
          name={part.function_name}
          args={part.arguments}
          response={toolResponses.get(part.tool_call_id) ?? null}
        />
      );
    case "refusal":
      return (
        <>
          <span className={cn("text-info-dim")}>refusal</span>
          <MediaView media={{ kind: "text", text: part.text }} />
        </>
      );
    case "text":
      return <MediaView media={{ kind: "text", text: part.text }} />;
    case "image":
      return <MediaView media={{ kind: "image", url: part.image.url }} />;
    case "audio":
      return (
        <MediaView
          media={{
            kind: "audio",
            data: part.audio.data,
            format: part.audio.format,
          }}
        />
      );
    case "video":
      return <MediaView media={{ kind: "video", url: part.video.url }} />;
    case "file":
      return <MediaView media={{ kind: "file", ...part.file }} />;
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

/** The normalized media descriptor every inlined content maps to, so
 * one renderer ([`MediaView`]) serves everything. */
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

/** An inlined `PartContent` → [`Media`]. */
function contentToMedia(content: PartContent): Media {
  switch (content.type) {
    case "text":
      return { kind: "text", text: content.text };
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
      // LOUD on purpose: a new content kind must be handled here, not
      // silently mislabeled.
      throw new Error(
        `unhandled part content kind: ${String(
          (content as { type: unknown }).type,
        )}`,
      );
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
