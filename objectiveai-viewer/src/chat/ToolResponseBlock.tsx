import cn from "classnames";
import type {
  AgentCompletionsMessageRichContent,
  AgentCompletionsMessageRichContentPart,
} from "@objectiveai/sdk";
import { JsonBlock } from "./JsonBlock";

interface ToolResponseBlockProps {
  toolName: string;
  content: AgentCompletionsMessageRichContent;
  dropdownKey: string;
  overrides: Record<string, boolean>;
  setOverride: (key: string, open: boolean) => void;
}

export function ToolResponseBlock({
  toolName,
  content,
  dropdownKey,
  overrides,
  setOverride,
}: ToolResponseBlockProps) {
  const open = overrides[dropdownKey] ?? false;

  return (
    <div
      className={cn(
        "rounded",
        "bg-neutral-50",
        "dark:bg-neutral-900",
        "border",
        "border-neutral-200",
        "dark:border-neutral-800",
        "mt-1",
        "ml-4",
      )}
    >
      <button
        type="button"
        onClick={() => setOverride(dropdownKey, !open)}
        className={cn(
          "w-full",
          "flex",
          "items-center",
          "gap-1",
          "px-2",
          "py-1",
          "text-xs",
          "text-neutral-700",
          "dark:text-neutral-300",
          "cursor-pointer",
          "select-none",
          "hover:bg-neutral-100",
          "dark:hover:bg-neutral-800",
          "rounded",
        )}
        aria-expanded={open}
      >
        <span className={cn("w-3", "text-center")}>{open ? "▾" : "▸"}</span>
        <span className={cn("font-semibold")}>{toolName}</span>
        <span className={cn("text-neutral-500", "dark:text-neutral-400")}>
          response
        </span>
      </button>
      {open && (
        <div
          className={cn(
            "px-2",
            "py-2",
            "border-t",
            "border-neutral-200",
            "dark:border-neutral-800",
          )}
        >
          <ResponseBody content={content} />
        </div>
      )}
    </div>
  );
}

function ResponseBody({ content }: { content: AgentCompletionsMessageRichContent }) {
  if (typeof content === "string") {
    return <TextOrJson text={content} />;
  }
  if (Array.isArray(content)) {
    if (content.length === 0) {
      return (
        <div
          className={cn(
            "text-xs",
            "italic",
            "text-neutral-500",
            "dark:text-neutral-400",
          )}
        >
          (empty)
        </div>
      );
    }
    return (
      <div className={cn("flex", "flex-col", "gap-2")}>
        {content.map((part, i) => (
          <PartView key={i} part={part} />
        ))}
      </div>
    );
  }
  return null;
}

function PartView({ part }: { part: AgentCompletionsMessageRichContentPart }) {
  if ("text" in part && typeof part.text === "string") {
    return <TextOrJson text={part.text} />;
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
          "rounded",
          "border",
          "border-neutral-200",
          "dark:border-neutral-700",
        )}
      />
    );
  }
  if ("file" in part) {
    const file = (part as { file: { filename?: string } }).file;
    return (
      <span
        className={cn(
          "inline-block",
          "rounded",
          "bg-neutral-200",
          "dark:bg-neutral-800",
          "px-2",
          "py-1",
          "text-xs",
        )}
      >
        {file.filename ?? "File"}
      </span>
    );
  }
  return null;
}

function TextOrJson({ text }: { text: string }) {
  const trimmed = text.trim();
  if (trimmed.length === 0) {
    return (
      <div
        className={cn(
          "text-xs",
          "italic",
          "text-neutral-500",
          "dark:text-neutral-400",
        )}
      >
        (empty)
      </div>
    );
  }
  if (
    (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
    (trimmed.startsWith("[") && trimmed.endsWith("]"))
  ) {
    try {
      const parsed = JSON.parse(trimmed);
      return <JsonBlock value={parsed} />;
    } catch {
      // fall through to text
    }
  }
  return (
    <pre
      className={cn(
        "text-xs",
        "whitespace-pre-wrap",
        "break-all",
        "text-neutral-700",
        "dark:text-neutral-300",
      )}
    >
      {text}
    </pre>
  );
}
