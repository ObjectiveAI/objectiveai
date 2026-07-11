import cn from "classnames";
import { LogoMark } from "./shared/Logo";

/**
 * The `laboratories` home pane — the future laboratory builder
 * (create / list / connect, on top of the Tauri laboratory
 * commands). Placeholder body for now: the pane and its tab landed
 * first so the builder has a home.
 */
export function LaboratoriesPane() {
  return (
    <div
      className={cn(
        "relative",
        "flex-1",
        "min-h-0",
        "flex",
        "flex-col",
        "items-center",
        "justify-center",
        "gap-3",
        "select-none",
      )}
    >
      <LogoMark className={cn("h-24", "w-auto", "text-info-dim/15")} />
      <span className={cn("font-mono", "text-sm", "text-info-dim")}>
        laboratory builder — coming soon
      </span>
    </div>
  );
}
