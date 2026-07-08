import { useEffect, useState, useSyncExternalStore } from "react";
import cn from "classnames";
import {
  clearErrors,
  installGlobalErrorHandlers,
  reportedErrors,
  subscribeErrors,
  type ReportedError,
} from "../lib/errors";

/**
 * The bottom-right error surface: invisible while the inbox is empty;
 * otherwise a small clickable badge (`⚠ N`) that expands into a panel
 * listing every reported error — source, first-seen time, repeat
 * count, message, and (per entry, expandable) the stack/detail.
 * Errors that would otherwise vanish into silent catches land here
 * via [`reportError`](../lib/errors.ts); uncaught window errors and
 * unhandled rejections are captured globally on mount.
 */
export function ErrorToast() {
  const errors = useSyncExternalStore(subscribeErrors, reportedErrors);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    installGlobalErrorHandlers();
  }, []);

  if (errors.length === 0) return null;
  const total = errors.reduce((sum, error) => sum + error.count, 0);
  return (
    <div
      data-error-toast
      className={cn(
        "fixed",
        "bottom-10",
        "right-3",
        "z-[60]",
        "flex",
        "flex-col",
        "items-end",
        "gap-1.5",
        "font-mono",
      )}
    >
      {open && (
        <div
          data-error-panel
          className={cn(
            "w-[420px]",
            "max-w-[80vw]",
            "max-h-[50vh]",
            "overflow-auto",
            "flex",
            "flex-col",
            "rounded-md",
            "border",
            "border-red-800",
            "bg-ground",
            "shadow-[0_0_16px_rgba(153,27,27,0.45)]",
          )}
        >
          <div
            className={cn(
              "flex",
              "items-center",
              "gap-2",
              "px-3",
              "py-1.5",
              "border-b",
              "border-red-900/60",
              "text-[10px]",
              "text-red-300",
              "shrink-0",
            )}
          >
            <span>
              {total} error{total === 1 ? "" : "s"}
            </span>
            <button
              type="button"
              data-error-clear
              onClick={() => {
                clearErrors();
                setOpen(false);
              }}
              className={cn(
                "ml-auto",
                "px-1.5",
                "rounded-sm",
                "text-red-400",
                "hover:text-red-200",
                "cursor-pointer",
              )}
            >
              clear
            </button>
          </div>
          <div className={cn("flex", "flex-col", "px-3", "py-2", "gap-2")}>
            {[...errors].reverse().map((error) => (
              <ErrorEntry key={error.id} error={error} />
            ))}
          </div>
        </div>
      )}
      <button
        type="button"
        data-error-badge
        onClick={() => setOpen((v) => !v)}
        aria-label={open ? "Hide errors" : "Show errors"}
        className={cn(
          "flex",
          "items-center",
          "gap-1.5",
          "px-2.5",
          "py-1",
          "rounded-full",
          "border",
          "border-red-800",
          "bg-ground",
          "text-red-300",
          "hover:text-red-100",
          "shadow-[0_0_10px_rgba(153,27,27,0.5)]",
          "cursor-pointer",
          "text-[10px]",
          "tabular-nums",
          "select-none",
        )}
      >
        <span aria-hidden>⚠</span>
        <span>{total}</span>
      </button>
    </div>
  );
}

/** One inbox entry: header line + expandable detail. */
function ErrorEntry({ error }: { error: ReportedError }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div
      data-error-entry
      className={cn("flex", "flex-col", "gap-0.5", "text-[10px]")}
    >
      <button
        type="button"
        onClick={() => setExpanded((v) => !v)}
        className={cn(
          "flex",
          "items-baseline",
          "gap-2",
          "text-left",
          error.detail !== null && "cursor-pointer",
        )}
      >
        <span className={cn("text-red-400", "shrink-0")}>{error.source}</span>
        <span className={cn("text-info-dim", "shrink-0", "tabular-nums")}>
          {error.at}
          {error.count > 1 ? ` ×${error.count}` : ""}
        </span>
        <span className={cn("text-[#c3bfbb]", "break-words", "min-w-0")}>
          {error.message}
        </span>
      </button>
      {expanded && error.detail !== null && (
        <pre
          className={cn(
            "text-[9px]",
            "text-info-mid",
            "whitespace-pre-wrap",
            "break-words",
            "leading-snug",
            "border-l",
            "border-red-900/60",
            "pl-2",
            "select-text",
          )}
        >
          {error.detail}
        </pre>
      )}
    </div>
  );
}
