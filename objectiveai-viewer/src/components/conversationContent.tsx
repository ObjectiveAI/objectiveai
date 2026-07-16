/**
 * Shared per-kind rendering for conversation content — the mirror
 * blocks of the daemon's `/agents/instances/{*aih}` stream (the
 * `agents logs list` `ResponseItem` family with content inlined).
 * Used by the hierarchy tree's latest-item box (last part only) and
 * the conversation popup (every part): one place decides how each
 * content permutation displays and what its indicator label says.
 */
import cn from "classnames";
import type {
  CliAgentsInstancesListenerAssistantResponsePart,
  CliAgentsInstancesListenerPartContent,
} from "@objectiveai/sdk";
import { Markdown } from "./Markdown";
import { JsonBlock } from "./shared/JsonBlock";
import type { ConversationBlock } from "../hooks/useAgentInstance";

export type PartContent = CliAgentsInstancesListenerPartContent;
export type AssistantPart = CliAgentsInstancesListenerAssistantResponsePart;

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
    case "error":
      return "error";
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

/** Render one media content per kind, labeled `{scope} · {kind}`. */
export function describeContent(
  scope: string,
  content: PartContent,
): { label: string; body: React.ReactNode } {
  switch (content.type) {
    case "text":
      return {
        label: `${scope} · text`,
        body: <Markdown>{content.text}</Markdown>,
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

/** Render one assistant-family part per kind — tool calls surface the
 * tool NAME in the label and their arguments as formatted JSON. */
export function describeAssistantPart(
  scope: string,
  part: AssistantPart,
): { label: string; body: React.ReactNode } {
  switch (part.type) {
    case "text":
      return {
        label: `${scope} · text`,
        body: <Markdown>{part.text}</Markdown>,
      };
    case "reasoning":
      return {
        label: `${scope} · reasoning`,
        body: <Markdown>{part.text}</Markdown>,
      };
    case "refusal":
      return {
        label: `${scope} · refusal`,
        body: <Markdown>{part.text}</Markdown>,
      };
    case "tool_call":
      return {
        label: `tool call · ${part.function_name}`,
        body: <JsonBody value={part.arguments} />,
      };
    case "image":
      return {
        label: `${scope} · image`,
        body: <UrlBody url={part.image.url} />,
      };
    case "audio":
      return {
        label: `${scope} · audio`,
        body: <span>[audio · {part.audio.format}]</span>,
      };
    case "video":
      return {
        label: `${scope} · video`,
        body: <UrlBody url={part.video.url} />,
      };
    case "file": {
      const label =
        part.file.filename ?? part.file.file_url ?? part.file.file_id ??
        "[file]";
      return { label: `${scope} · file`, body: <span>{label}</span> };
    }
    default:
      throw new Error(
        `unhandled assistant part kind: ${String(
          (part as { type: unknown }).type,
        )}`,
      );
  }
}

/** What a block's LATEST item is (the tree's indicator + body): the
 * last part, per kind — or the block's own inline value for the
 * single-row kinds (vote / error). */
export function describeLastItem(block: ConversationBlock): {
  label: string;
  body: React.ReactNode;
} {
  const scope = blockScope(block);
  switch (block.type) {
    case "vector_response_vote":
      return { label: scope, body: <JsonBody value={block.vote} /> };
    case "error":
      return { label: scope, body: <JsonBody value={block.error} /> };
    case "assistant_response":
    case "request_message_assistant": {
      const part = block.parts[block.parts.length - 1];
      if (part === undefined) return { label: scope, body: null };
      return describeAssistantPart(scope, part);
    }
    case "vector_request_choices": {
      const choice = block.choices[block.choices.length - 1];
      const part = choice?.parts[choice.parts.length - 1];
      if (part === undefined) return { label: scope, body: null };
      return describeContent(scope, part.content);
    }
    default: {
      const part = block.parts[block.parts.length - 1];
      if (part === undefined) return { label: scope, body: null };
      return describeContent(scope, part.content);
    }
  }
}

/** Pretty-printed JSON body: a string that parses as JSON renders
 * re-indented (string-embedded JSON expanded, wrapped lines hang at
 * their own indentation — [`JsonBlock`]); anything else renders raw. */
export function JsonBody({ value }: { value: unknown }) {
  return (
    <JsonBlock
      value={value}
      className={cn("text-[9px]", "leading-snug")}
    />
  );
}

export function UrlBody({ url }: { url: string }) {
  return <span className={cn("break-all")}>{url}</span>;
}
