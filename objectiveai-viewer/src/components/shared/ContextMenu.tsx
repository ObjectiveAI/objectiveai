import { useEffect, useRef, type ReactElement, type ReactNode } from "react";
import { createPortal } from "react-dom";
import cn from "classnames";

/**
 * The shared right-click menu: a small bordered popover PORTALED to
 * `document.body` and fixed at the cursor's viewport coordinates —
 * portaling matters because the hierarchy canvas renders under a CSS
 * `zoom`, which would rescale an in-tree fixed element and land the
 * menu away from the cursor. Dismisses on any pointer-down outside,
 * on Escape, and on window blur. Content is the caller's —
 * [`ContextMenuItem`] rows, or anything else (an inline input swap).
 */
export function ContextMenu({
  position,
  onClose,
  children,
}: {
  position: { x: number; y: number };
  onClose: () => void;
  children?: ReactNode;
}): ReactElement {
  const ref = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const onPointerDown = (event: PointerEvent) => {
      if (ref.current !== null && !ref.current.contains(event.target as Node)) {
        onClose();
      }
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    const onBlur = () => onClose();
    // Capture phase so a click that something else swallows still
    // dismisses the menu.
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", onBlur);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", onBlur);
    };
  }, [onClose]);

  return createPortal(
    <div
      ref={ref}
      data-context-menu
      style={{ left: position.x, top: position.y }}
      className={cn(
        "fixed",
        "z-[70]",
        "min-w-40",
        "py-1",
        "rounded-sm",
        "border",
        "border-copper-mid",
        "bg-ground-surface",
        "shadow-[0_0_12px_rgba(0,0,0,0.5)]",
        "font-mono",
        "text-xs",
        "text-info-mid",
        "select-none",
      )}
    >
      {children}
    </div>,
    document.body,
  );
}

/** One menu row. Disabled rows render dim and inert. */
export function ContextMenuItem({
  label,
  disabled,
  onSelect,
  dataAttr,
}: {
  label: string;
  disabled?: boolean;
  onSelect: () => void;
  dataAttr?: string;
}): ReactElement {
  return (
    <button
      type="button"
      disabled={disabled}
      {...(dataAttr !== undefined ? { [dataAttr]: true } : {})}
      onClick={onSelect}
      className={cn(
        "block",
        "w-full",
        "px-3",
        "py-1",
        "text-left",
        disabled
          ? "text-info-dim/60"
          : cn("cursor-pointer", "hover:bg-copper-warm/10", "hover:text-copper-bright"),
      )}
    >
      {label}
    </button>
  );
}
