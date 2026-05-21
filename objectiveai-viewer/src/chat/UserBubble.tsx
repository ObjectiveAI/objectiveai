import cn from "classnames";
import type {
  AgentCompletionsMessageMessage,
  AgentCompletionsMessageRichContentPart,
} from "@objectiveai/sdk";

interface UserBubbleProps {
  msg: AgentCompletionsMessageMessage;
}

export function UserBubble({ msg }: UserBubbleProps) {
  const m = msg as { role: string; content: unknown };
  if (m.role !== "user") return null;
  return (
    <div
      className={cn(
        "self-end",
        "max-w-[85%]",
        "rounded-2xl",
        "rounded-br-sm",
        "bg-blue-600",
        "text-white",
        "px-3",
        "py-2",
        "text-sm",
        "shadow-sm",
        "whitespace-pre-wrap",
        "break-words",
      )}
    >
      <RichContent content={m.content} />
    </div>
  );
}

function RichContent({ content }: { content: unknown }) {
  if (typeof content === "string") {
    return <>{content}</>;
  }
  if (Array.isArray(content)) {
    return (
      <div className={cn("flex", "flex-col", "gap-2")}>
        {content.map((part, i) => (
          <RichContentPart key={i} part={part as AgentCompletionsMessageRichContentPart} />
        ))}
      </div>
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
          "border-blue-400",
        )}
      />
    );
  }
  if ("input_audio" in part) {
    return <AttachmentBadge label="Audio" />;
  }
  if ("video_url" in part || "input_video" in part) {
    return <AttachmentBadge label="Video" />;
  }
  if ("file" in part) {
    const file = (part as { file: { filename?: string } }).file;
    return <AttachmentBadge label={file.filename ?? "File"} />;
  }
  return null;
}

function AttachmentBadge({ label }: { label: string }) {
  return (
    <span
      className={cn(
        "inline-block",
        "rounded",
        "bg-blue-700",
        "px-2",
        "py-1",
        "text-xs",
      )}
    >
      {label}
    </span>
  );
}
