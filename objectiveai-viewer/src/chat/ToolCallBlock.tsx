import cn from "classnames";
import type { AgentCompletionsMessageAssistantToolCallDelta } from "@objectiveai/sdk";
import { JsonBlock } from "./JsonBlock";

interface ToolCallBlockProps {
  call: AgentCompletionsMessageAssistantToolCallDelta;
  dropdownKey: string;
  overrides: Record<string, boolean>;
  setOverride: (key: string, open: boolean) => void;
}

export function ToolCallBlock({
  call,
  dropdownKey,
  overrides,
  setOverride,
}: ToolCallBlockProps) {
  const open = overrides[dropdownKey] ?? false;
  const name = call.function?.name ?? "unknown";
  const rawArgs = call.function?.arguments ?? "";

  return (
    <div
      className={cn(
        "rounded",
        "bg-neutral-50",
        "dark:bg-neutral-900",
        "border",
        "border-neutral-200",
        "dark:border-neutral-800",
        "mt-2",
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
        <span className={cn("font-semibold")}>{name}</span>
        <span className={cn("text-neutral-500", "dark:text-neutral-400")}>
          call
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
          <CallArgs raw={rawArgs} />
        </div>
      )}
    </div>
  );
}

function CallArgs({ raw }: { raw: string }) {
  if (raw.trim().length === 0) {
    return (
      <div
        className={cn(
          "text-xs",
          "italic",
          "text-neutral-500",
          "dark:text-neutral-400",
        )}
      >
        (no arguments)
      </div>
    );
  }
  try {
    const parsed = JSON.parse(raw);
    return <JsonBlock value={parsed} />;
  } catch {
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
        {raw}
      </pre>
    );
  }
}
