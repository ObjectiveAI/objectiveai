import cn from "classnames";

/**
 * The corner-tab `open ↗` button — the ONE opener style shared by the
 * agent box and the laboratory card. Visual classes only: callers
 * supply positioning (`self-*` plus the negative margins that pull
 * the tab through their own box padding so its border merges 1px with
 * the box's top-right corner — those offsets depend on each host
 * box's padding/border, so they don't belong here). Rounded only
 * where it matches the box corner (tr) and where it faces the
 * content (bl).
 */
export function OpenTab({
  onClick,
  ariaLabel,
  dataAttr,
  className,
}: {
  onClick: () => void;
  ariaLabel: string;
  dataAttr: "data-open-agent" | "data-open-laboratory";
  className?: string;
}) {
  return (
    <button
      type="button"
      {...{ [dataAttr]: true }}
      onClick={onClick}
      aria-label={ariaLabel}
      className={cn(
        "rounded-tr-sm",
        "rounded-bl-sm",
        "rounded-tl-none",
        "rounded-br-none",
        "px-1.5",
        "py-px",
        "border",
        "border-copper-mid",
        "bg-copper-warm/10",
        "text-copper-bright",
        "text-xs",
        "hover:border-copper-hot",
        "hover:text-copper-hot",
        "cursor-pointer",
        className,
      )}
    >
      open ↗
    </button>
  );
}
