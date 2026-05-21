import cn from "classnames";
import type {
  AgentCompletionsMessageRichContentPart,
  AgentCompletionsResponseStreamingAssistantResponseChunk,
} from "@objectiveai/sdk";
import { ReasoningBlock } from "./ReasoningBlock";
import { ToolCallBlock } from "./ToolCallBlock";
import { reasoningKey, toolCallKey } from "./dropdownKey";

interface AssistantBubbleProps {
  msg: AgentCompletionsResponseStreamingAssistantResponseChunk;
  streaming?: boolean;
  entryRequestId: string;
  msgIndex: number;
  overrides: Record<string, boolean>;
  setOverride: (key: string, open: boolean) => void;
}

export function AssistantBubble({
  msg,
  streaming = false,
  entryRequestId,
  msgIndex,
  overrides,
  setOverride,
}: AssistantBubbleProps) {
  const pulseHere = streaming && !msg.finish_reason;
  const hasReasoning = !!msg.reasoning && msg.reasoning.length > 0;
  const hasContent = !isEmptyRichContent(msg.content);
  const hasToolCalls = (msg.tool_calls?.length ?? 0) > 0;
  const reasoningDefaultOpen = !hasContent && !hasToolCalls;

  return (
    <div
      className={cn(
        "self-start",
        "max-w-[85%]",
        "rounded-2xl",
        "rounded-bl-sm",
        "bg-white",
        "dark:bg-neutral-800",
        "text-neutral-900",
        "dark:text-neutral-50",
        "px-3",
        "py-2",
        "text-sm",
        "shadow-sm",
        "border",
        "border-neutral-200",
        "dark:border-neutral-700",
        "whitespace-pre-wrap",
        "break-words",
      )}
    >
      {hasReasoning && (
        <ReasoningBlock
          text={msg.reasoning ?? ""}
          dropdownKey={reasoningKey(entryRequestId, msgIndex)}
          defaultOpen={reasoningDefaultOpen}
          overrides={overrides}
          setOverride={setOverride}
        />
      )}

      {msg.refusal && (
        <div
          className={cn(
            "rounded",
            "bg-amber-50",
            "dark:bg-amber-950",
            "text-amber-800",
            "dark:text-amber-200",
            "border",
            "border-amber-200",
            "dark:border-amber-800",
            "px-2",
            "py-1",
            "text-xs",
            "mb-2",
          )}
        >
          {msg.refusal}
        </div>
      )}

      {hasContent && <RichContent content={msg.content as unknown} />}

      {pulseHere && (
        <span
          className={cn(
            "inline-block",
            "ml-1",
            "w-1.5",
            "h-3",
            "bg-neutral-400",
            "dark:bg-neutral-500",
            "align-middle",
            "animate-pulse",
          )}
          aria-label="streaming"
        />
      )}

      {hasToolCalls &&
        msg.tool_calls!.map((call, ci) => (
          <ToolCallBlock
            key={ci}
            call={call}
            dropdownKey={toolCallKey(entryRequestId, msgIndex, ci)}
            overrides={overrides}
            setOverride={setOverride}
          />
        ))}
    </div>
  );
}

function isEmptyRichContent(content: unknown): boolean {
  if (content === null || content === undefined) return true;
  if (typeof content === "string") return content.trim().length === 0;
  if (Array.isArray(content)) {
    if (content.length === 0) return true;
    return content.every((part) => {
      if (typeof part !== "object" || part === null) return true;
      if ("text" in part && typeof (part as { text: unknown }).text === "string") {
        return (part as { text: string }).text.trim().length === 0;
      }
      return false; // image, file, audio, video etc. count as content
    });
  }
  return false;
}

function RichContent({ content }: { content: unknown }) {
  if (typeof content === "string") {
    return <>{content}</>;
  }
  if (Array.isArray(content)) {
    return (
      <>
        {content.map((part, i) => (
          <RichContentPart key={i} part={part as AgentCompletionsMessageRichContentPart} />
        ))}
      </>
    );
  }
  return null;
}

function RichContentPart({ part }: { part: AgentCompletionsMessageRichContentPart }) {
  if ("text" in part && typeof part.text === "string") {
    return <span>{part.text}</span>;
  }
  if ("image_url" in part) {
    const url = (part as { image_url: { url: string } }).image_url.url;
    return (
      <img
        src={url}
        alt=""
        className={cn(
          "max-w-full",
          "max-h-48",
          "object-contain",
          "rounded-md",
          "border",
          "border-neutral-200",
          "dark:border-neutral-700",
        )}
      />
    );
  }
  return null;
}
