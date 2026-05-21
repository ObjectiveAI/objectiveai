import { useRef, type KeyboardEvent, type ChangeEvent } from "react";
import cn from "classnames";
import type { AgentCompletionsMessageRichContentPart } from "@objectiveai/sdk";
import { fileToRichContentPart } from "./fileToRichContent";

interface ChatPaneProps {
  value: string;
  onChange: (value: string) => void;
  attachments: AgentCompletionsMessageRichContentPart[];
  onAttachmentsChange: (
    next: AgentCompletionsMessageRichContentPart[],
  ) => void;
  onSend: () => void;
  disabled?: boolean;
}

export function ChatPane({
  value,
  onChange,
  attachments,
  onAttachmentsChange,
  onSend,
  disabled = false,
}: ChatPaneProps) {
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const trySend = () => {
    if (disabled) return;
    if (value.trim().length === 0 && attachments.length === 0) return;
    onSend();
  };

  const onKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      trySend();
    }
  };

  const onFilesPicked = async (e: ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    const fileArray = Array.from(files);
    e.target.value = "";

    const parts = await Promise.all(
      fileArray.map((f) =>
        fileToRichContentPart(f).catch((err) => {
          // eslint-disable-next-line no-console
          console.warn("failed to convert file", f.name, err);
          return null;
        }),
      ),
    );
    const good = parts.filter(
      (p): p is AgentCompletionsMessageRichContentPart => p !== null,
    );
    if (good.length > 0) onAttachmentsChange([...attachments, ...good]);
  };

  const removeAttachment = (index: number) => {
    const next = attachments.slice();
    next.splice(index, 1);
    onAttachmentsChange(next);
  };

  return (
    <div
      className={cn(
        "flex",
        "flex-col",
        "border-t",
        "border-neutral-300",
        "dark:border-neutral-700",
        "bg-neutral-50",
        "dark:bg-neutral-900",
        "p-2",
        "gap-2",
      )}
    >
      {attachments.length > 0 && (
        <div
          className={cn(
            "flex",
            "flex-row",
            "gap-2",
            "overflow-x-auto",
            "pb-1",
          )}
        >
          {attachments.map((part, i) => (
            <AttachmentPill
              key={i}
              part={part}
              onRemove={() => removeAttachment(i)}
            />
          ))}
        </div>
      )}

      <div className={cn("flex", "flex-row", "items-end", "gap-2")}>
        <button
          type="button"
          onClick={() => fileInputRef.current?.click()}
          disabled={disabled}
          aria-label="Attach files"
          className={cn(
            "shrink-0",
            "h-10",
            "w-10",
            "rounded-md",
            "border",
            "border-neutral-300",
            "dark:border-neutral-600",
            "bg-white",
            "dark:bg-neutral-800",
            "text-neutral-700",
            "dark:text-neutral-300",
            "hover:bg-neutral-100",
            "dark:hover:bg-neutral-700",
            "cursor-pointer",
            "disabled:cursor-not-allowed",
            "disabled:opacity-50",
            "flex",
            "items-center",
            "justify-center",
          )}
        >
          {"\u{1F4CE}"}
        </button>

        <input
          ref={fileInputRef}
          type="file"
          multiple
          accept="*/*"
          onChange={onFilesPicked}
          className={cn("hidden")}
        />

        <textarea
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={onKeyDown}
          rows={1}
          placeholder="Message…"
          disabled={disabled}
          className={cn(
            "flex-1",
            "min-h-10",
            "max-h-48",
            "resize-none",
            "rounded-md",
            "border",
            "border-neutral-300",
            "dark:border-neutral-600",
            "bg-white",
            "dark:bg-neutral-800",
            "text-neutral-900",
            "dark:text-neutral-100",
            "px-3",
            "py-2",
            "text-sm",
            "focus:outline-none",
            "focus:border-blue-600",
            "dark:focus:border-blue-400",
            "disabled:cursor-not-allowed",
            "disabled:opacity-50",
          )}
        />

        <button
          type="button"
          onClick={trySend}
          disabled={
            disabled || (value.trim().length === 0 && attachments.length === 0)
          }
          aria-label="Send"
          className={cn(
            "shrink-0",
            "h-10",
            "w-10",
            "rounded-md",
            "bg-blue-600",
            "text-white",
            "hover:bg-blue-700",
            "cursor-pointer",
            "disabled:cursor-not-allowed",
            "disabled:opacity-50",
            "flex",
            "items-center",
            "justify-center",
          )}
        >
          {"➤"}
        </button>
      </div>
    </div>
  );
}

interface AttachmentPillProps {
  part: AgentCompletionsMessageRichContentPart;
  onRemove: () => void;
}

function AttachmentPill({ part, onRemove }: AttachmentPillProps) {
  const label = describePart(part);
  const thumb = thumbnailUrl(part);

  return (
    <div
      className={cn(
        "flex",
        "flex-row",
        "items-center",
        "gap-2",
        "shrink-0",
        "rounded-md",
        "border",
        "border-neutral-300",
        "dark:border-neutral-600",
        "bg-white",
        "dark:bg-neutral-800",
        "px-2",
        "py-1",
        "text-xs",
        "text-neutral-700",
        "dark:text-neutral-300",
      )}
    >
      {thumb ? (
        <img
          src={thumb}
          alt=""
          className={cn(
            "h-8",
            "w-8",
            "object-cover",
            "rounded",
            "border",
            "border-neutral-200",
            "dark:border-neutral-700",
          )}
        />
      ) : (
        <span
          className={cn(
            "h-8",
            "w-8",
            "rounded",
            "bg-neutral-200",
            "dark:bg-neutral-700",
            "flex",
            "items-center",
            "justify-center",
            "text-base",
          )}
        >
          {iconForPart(part)}
        </span>
      )}
      <span className={cn("max-w-32", "truncate")}>{label}</span>
      <button
        type="button"
        onClick={onRemove}
        aria-label={`Remove ${label}`}
        className={cn(
          "shrink-0",
          "h-5",
          "w-5",
          "rounded",
          "text-neutral-500",
          "dark:text-neutral-400",
          "hover:bg-neutral-200",
          "dark:hover:bg-neutral-700",
          "cursor-pointer",
          "flex",
          "items-center",
          "justify-center",
        )}
      >
        {"×"}
      </button>
    </div>
  );
}

function describePart(part: AgentCompletionsMessageRichContentPart): string {
  switch (part.type) {
    case "text":
      return "Text";
    case "image_url":
      return "Image";
    case "video_url":
    case "input_video":
      return "Video";
    case "input_audio":
      return `Audio (${part.input_audio.format})`;
    case "file":
      return part.file.filename ?? "File";
  }
}

function thumbnailUrl(
  part: AgentCompletionsMessageRichContentPart,
): string | null {
  if (part.type === "image_url") return part.image_url.url;
  return null;
}

function iconForPart(part: AgentCompletionsMessageRichContentPart): string {
  switch (part.type) {
    case "video_url":
    case "input_video":
      return "\u{1F39E}";
    case "input_audio":
      return "\u{1F50A}";
    case "file":
      return "\u{1F4C4}";
    default:
      return "\u{1F4CE}";
  }
}
