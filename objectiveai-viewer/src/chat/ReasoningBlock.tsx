import cn from "classnames";

interface ReasoningBlockProps {
  text: string;
  dropdownKey: string;
  defaultOpen: boolean;
  overrides: Record<string, boolean>;
  setOverride: (key: string, open: boolean) => void;
}

export function ReasoningBlock({
  text,
  dropdownKey,
  defaultOpen,
  overrides,
  setOverride,
}: ReasoningBlockProps) {
  const open = overrides[dropdownKey] ?? defaultOpen;
  return (
    <div
      className={cn(
        "rounded",
        "bg-neutral-100",
        "dark:bg-neutral-900",
        "border",
        "border-neutral-200",
        "dark:border-neutral-800",
        "mb-2",
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
          "text-neutral-600",
          "dark:text-neutral-400",
          "cursor-pointer",
          "select-none",
          "hover:bg-neutral-200",
          "dark:hover:bg-neutral-800",
          "rounded",
        )}
        aria-expanded={open}
      >
        <span className={cn("w-3", "text-center")}>{open ? "▾" : "▸"}</span>
        <span>Reasoning</span>
      </button>
      {open && (
        <div
          className={cn(
            "px-2",
            "py-2",
            "text-xs",
            "text-neutral-600",
            "dark:text-neutral-400",
            "whitespace-pre-wrap",
            "border-t",
            "border-neutral-200",
            "dark:border-neutral-800",
          )}
        >
          {text}
        </div>
      )}
    </div>
  );
}
