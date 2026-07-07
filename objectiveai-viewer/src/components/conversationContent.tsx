/**
 * Shared per-kind rendering for conversation rows — the inlined
 * content of the daemon's `/agents/instances/{*aih}` stream. Used by
 * the hierarchy tree's latest-item box (last row only) and the
 * conversation popup (every row): one place decides how each content
 * permutation displays and what its indicator label says. Tool calls
 * surface the tool NAME in the label and their arguments as formatted
 * JSON; text-ish kinds render as markdown; media kinds render as
 * URL/format/filename lines.
 */
import cn from "classnames";
import { Markdown } from "./Markdown";
import type {
  ConversationBlock,
  ConversationRow,
} from "../hooks/useAgentInstance";

/** The block-level scope label — the first half of every indicator.
 * Deliberately coarse: request-vs-response is not distinguished
 * (`assistant` covers both; `tool` covers every tool shape). */
export function blockScope(block: ConversationBlock): string {
  switch (block.type) {
    case "assistant_response":
    case "request_message_assistant":
      return "assistant";
    case "tool_response":
    case "request_message_tool":
      return "tool";
    case "request_message_user":
      return "user";
    case "client_notification":
      return "notification";
    case "vector_request_choices":
      return "choices";
    case "vector_response_vote":
      return "vote";
    default:
      // LOUD on purpose: a new block kind must be handled here, not
      // silently mislabeled.
      throw new Error(
        `unhandled conversation block kind: ${String(
          (block as { type: unknown }).type,
        )}`,
      );
  }
}

/** Render one row per content kind, labeled `{scope} · {kind}` — tool
 * calls surface the tool NAME in the label and their arguments as
 * formatted JSON. `body: null` means nothing presentable (heads). */
export function describeRow(
  scope: string,
  row: ConversationRow | null,
): { label: string; body: React.ReactNode } {
  if (row === null) {
    return { label: scope, body: null };
  }
  const content = row.content;
  switch (content.type) {
    case "text":
      return {
        label: `${scope} · text`,
        body: <Markdown>{content.text}</Markdown>,
      };
    case "reasoning":
      return {
        label: `${scope} · reasoning`,
        body: <Markdown>{content.text}</Markdown>,
      };
    case "refusal":
      return {
        label: `${scope} · refusal`,
        body: <Markdown>{content.text}</Markdown>,
      };
    case "tool_call":
      return {
        label: `tool call · ${content.function_name}`,
        body: <JsonBody value={content.arguments} />,
      };
    case "image":
      return { label: `${scope} · image`, body: <UrlBody url={content.url} /> };
    case "audio":
      return {
        label: `${scope} · audio`,
        body: <span>[audio · {content.format}]</span>,
      };
    case "video":
      return { label: `${scope} · video`, body: <UrlBody url={content.url} /> };
    case "file": {
      const label =
        content.filename ?? content.file_url ?? content.file_id ?? "[file]";
      return { label: `${scope} · file`, body: <span>{label}</span> };
    }
    case "vote":
      return {
        label: `${scope} · vote`,
        body: <JsonBody value={content.vote} />,
      };
    case "head":
      // Heads never materialize as parts — defensive only.
      return { label: scope, body: null };
    default:
      // LOUD on purpose: a new content kind must be handled here, not
      // silently mislabeled.
      throw new Error(
        `unhandled conversation content kind: ${String(
          (content as { type: unknown }).type,
        )}`,
      );
  }
}

/** Pretty-printed JSON body: a string that parses as JSON renders
 * re-indented; anything else renders raw. */
export function JsonBody({ value }: { value: unknown }) {
  let text: string;
  if (typeof value === "string") {
    try {
      text = JSON.stringify(JSON.parse(value), null, 2);
    } catch {
      text = value;
    }
  } else {
    text = JSON.stringify(value, null, 2);
  }
  return (
    <pre
      className={cn(
        "text-[9px]",
        "whitespace-pre-wrap",
        "break-words",
        "leading-snug",
      )}
    >
      {text}
    </pre>
  );
}

export function UrlBody({ url }: { url: string }) {
  return <span className={cn("break-all")}>{url}</span>;
}

/** The little copper indicator chip naming what an item is. */
export function KindChip({
  label,
  marker,
}: {
  label: string;
  marker?: string;
}) {
  return (
    <span
      {...(marker !== undefined ? { [marker]: "" } : {})}
      className={cn(
        "self-start",
        "px-1.5",
        "py-px",
        "rounded-sm",
        "border",
        "border-copper-mid/70",
        "bg-copper-warm/10",
        "text-copper-bright",
        "text-[9px]",
        "whitespace-nowrap",
      )}
    >
      {label}
    </span>
  );
}
