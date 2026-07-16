import { useState, type ReactElement, type ReactNode } from "react";
import cn from "classnames";

/**
 * The shared editable ATTACHED BOX for agent-node cards: its own
 * bordered box chained below the agent box (borders overlap 1px, top
 * corners squared — the same attachment chrome as the latest-message
 * box, whose top it squares in turn when it follows). A dim label row
 * carries an inline `+` add affordance; the caller renders one row
 * per entry. Built for the tags box and reused verbatim by the
 * laboratories attach/detach box — this component knows NOTHING about
 * what it lists.
 *
 * Add flow: `+` swaps to an inline text input; Enter commits (the
 * input clears and stays open for another entry), Escape or blur
 * closes it. `onAdd` failures are the caller's to surface (toast) —
 * the input just re-enables. The add affordance is hidden entirely
 * when `onAdd` is absent.
 */
export function EditBox({
  label,
  addPlaceholder,
  onAdd,
  children,
}: {
  label: string;
  addPlaceholder?: string;
  onAdd?: (value: string) => Promise<void>;
  children?: ReactNode;
}): ReactElement {
  const [adding, setAdding] = useState(false);
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  const commit = () => {
    const trimmed = value.trim();
    if (!trimmed || busy || onAdd === undefined) {
      return;
    }
    setBusy(true);
    void onAdd(trimmed).finally(() => {
      setBusy(false);
      setValue("");
    });
  };

  return (
    <div
      data-edit-box={label}
      className={cn(
        // Attached to the box above: borders overlap 1px and share
        // copper-mid, so the stack reads as one connected unit.
        "-mt-px",
        // Zero preferred width + min-w-full: the wrapper (sized by
        // the agent box) dictates this box's width exactly.
        "w-0",
        "min-w-full",
        "px-2.5",
        "py-1.5",
        "rounded-sm",
        "rounded-t-none",
        // Square the bottom IFF the latest-message box follows — it
        // carries the stack's bottom rounding then.
        "[&:has(+[data-latest-text])]:rounded-b-none",
        "border",
        "border-copper-mid",
        "bg-ground-surface",
        "text-xs",
        "text-left",
        "flex",
        "flex-col",
        "gap-0.5",
      )}
    >
      <div
        className={cn(
          "flex",
          "flex-row",
          "items-center",
          "gap-1.5",
          "text-info-dim",
          "select-none",
        )}
      >
        <span>{label}</span>
        {onAdd !== undefined && !adding && (
          <button
            type="button"
            data-edit-box-add={label}
            aria-label={`Add ${label}`}
            onClick={() => setAdding(true)}
            className={cn(
              "px-1",
              "rounded-sm",
              "border",
              "border-copper-mid/40",
              "text-copper-bright/70",
              "hover:border-copper-hot",
              "hover:text-copper-hot",
              "cursor-pointer",
              "leading-4",
            )}
          >
            +
          </button>
        )}
        {onAdd !== undefined && adding && (
          <input
            autoFocus
            data-edit-box-add-input={label}
            value={value}
            disabled={busy}
            placeholder={addPlaceholder}
            onChange={(e) => setValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                commit();
              } else if (e.key === "Escape") {
                setAdding(false);
                setValue("");
              }
            }}
            onBlur={() => {
              if (!busy) {
                setAdding(false);
                setValue("");
              }
            }}
            className={cn(
              "bg-transparent",
              "border-b",
              "border-copper-mid/60",
              "outline-none",
              "text-info-mid",
              "placeholder:text-info-dim/60",
              "w-36",
              busy && "opacity-50",
            )}
          />
        )}
      </div>
      {children}
    </div>
  );
}

/**
 * The per-row remove affordance: a small `×` that dims while its
 * removal is in flight. Failures are the caller's to surface.
 */
export function EditBoxRemove({
  ariaLabel,
  onRemove,
  dataAttr,
}: {
  ariaLabel: string;
  onRemove: () => Promise<void>;
  dataAttr?: string;
}): ReactElement {
  const [busy, setBusy] = useState(false);
  return (
    <button
      type="button"
      aria-label={ariaLabel}
      {...(dataAttr !== undefined ? { [dataAttr]: true } : {})}
      disabled={busy}
      onClick={() => {
        setBusy(true);
        void onRemove().finally(() => setBusy(false));
      }}
      className={cn(
        "px-1",
        "rounded-sm",
        "text-info-dim",
        "hover:text-copper-hot",
        "cursor-pointer",
        "leading-4",
        busy && "opacity-40",
      )}
    >
      ×
    </button>
  );
}
