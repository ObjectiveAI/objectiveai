import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import { useAgo } from "../hooks/useAgo";
import { isInlineImage, useLaboratoriesList } from "../hooks/useLaboratoriesList";
import { useMachineIdentity } from "../hooks/useMachineIdentity";
import {
  classifyLaboratories,
  type DisplayLaboratory,
  type ViewerSource,
} from "../lib/laboratories";
import { LogoMark } from "./shared/Logo";

/**
 * The `laboratories` home pane: the daemon's `/laboratories/list`
 * stream (the host registry — the whole laboratory universe), each
 * laboratory classified `local` / `remote` by comparing its serving
 * host's machine identity against this machine's (see
 * [`classifyLaboratories`]). All laboratory state lives HERE —
 * unshared with the agents tab.
 */
export function LaboratoriesPane({
  transport,
}: {
  transport: ViewerTransport | null;
  active: boolean;
}) {
  const daemon = useLaboratoriesList(transport);
  const machine = useMachineIdentity();
  const laboratories = classifyLaboratories(daemon, machine);

  if (laboratories.length === 0) {
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
          no laboratories
        </span>
      </div>
    );
  }

  return (
    <div className={cn("flex-1", "min-h-0", "overflow-auto", "font-mono")}>
      <div
        className={cn(
          "max-w-content",
          "mx-auto",
          "px-4",
          "py-4",
          "flex",
          "flex-col",
          "gap-3",
        )}
      >
        {laboratories.map((lab) => (
          <LaboratoryCard key={lab.id} lab={lab} />
        ))}
      </div>
    </div>
  );
}

/** Copper-badge tint per source — the same palette the tree uses,
 * with a distinct hue only for `remote` (which leaves this machine's
 * trust boundary). */
const SOURCE_CLASSES: Record<ViewerSource, string> = {
  local: cn(
    "border-copper-mid/70",
    "bg-copper-warm/10",
    "text-copper-bright",
  ),
  remote: cn("border-info-mid/40", "bg-info-mid/10", "text-info-mid"),
};

function LaboratoryCard({ lab }: { lab: DisplayLaboratory }) {
  // "" when createdAt is absent — formatAgo renders nothing for
  // unparsable input, so the row simply doesn't appear.
  const createdAgo = useAgo(
    lab.createdAt != null ? new Date(lab.createdAt * 1000).toISOString() : "",
  );
  return (
    <div
      data-laboratory={lab.id}
      className={cn(
        "flex",
        "flex-col",
        "gap-1.5",
        "px-2.5",
        "py-2",
        "rounded-sm",
        "border",
        "border-copper-mid",
        "bg-ground-surface",
        "shadow-[0_0_8px_rgba(217,119,6,0.3)]",
      )}
    >
      {/* Header: connected dot + id + source chip. */}
      <div className={cn("flex", "items-center", "gap-2")}>
        <span
          className={cn(
            "w-1.5",
            "h-1.5",
            "rounded-full",
            "shrink-0",
            lab.connected
              ? cn("bg-copper-hot", "animate-pulse")
              : "bg-info-dim",
          )}
          title={lab.connected ? "connected" : "not connected"}
        />
        <span
          className={cn(
            "flex-1",
            "min-w-0",
            "truncate",
            "text-sm",
            "text-info-bright",
          )}
        >
          {lab.id}
        </span>
        <span
          className={cn(
            "shrink-0",
            "px-1.5",
            "py-px",
            "rounded-sm",
            "border",
            "text-xs",
            SOURCE_CLASSES[lab.source],
          )}
          title={
            lab.machine
              ? `${lab.machine.hostname ?? lab.machine.id} (${lab.machine.os})`
              : undefined
          }
        >
          {lab.source}
        </span>
      </div>

      {/* Spec detail. */}
      <div className={cn("flex", "flex-col", "gap-1", "text-xs", "text-[#c3bfbb]")}>
        {isInlineImage(lab.image) ? (
          <div className={cn("flex", "flex-col", "gap-0.5")}>
            <span className={cn("text-info-dim")}>containerfile</span>
            <pre
              className={cn(
                "pl-2",
                "whitespace-pre-wrap",
                "break-all",
                "font-mono",
                "max-h-32",
                "overflow-y-auto",
              )}
            >
              {lab.image.containerfile}
            </pre>
          </div>
        ) : (
          <>
            <DetailRow label="registry" value={lab.image.registry} />
            <DetailRow label="name" value={lab.image.name} />
            {"tag" in lab.image ? (
              <DetailRow label="tag" value={lab.image.tag} />
            ) : (
              <DetailRow label="digest" value={lab.image.digest} />
            )}
          </>
        )}
        <DetailRow label="cwd" value={lab.cwd} />
        {lab.agentFullId !== null && (
          <DetailRow label="agent" value={lab.agentFullId} />
        )}
        {lab.mounts.length > 0 && (
          <div className={cn("flex", "flex-col", "gap-0.5")}>
            <span className={cn("text-info-dim")}>mounts</span>
            {lab.mounts.map((m, i) => (
              <span key={i} className={cn("pl-2", "break-all")}>
                {m.host} → {m.container}
              </span>
            ))}
          </div>
        )}
        {lab.env.length > 0 && (
          <div className={cn("flex", "flex-col", "gap-0.5")}>
            {/* Keys only — env values may be secrets. */}
            <span className={cn("text-info-dim")}>env</span>
            <div className={cn("flex", "flex-wrap", "gap-1", "pl-2")}>
              {lab.env.map((e) => (
                <span
                  key={e.key}
                  className={cn(
                    "px-1.5",
                    "py-px",
                    "rounded-sm",
                    "border",
                    "border-copper-mid/70",
                    "bg-copper-warm/10",
                    "text-copper-bright",
                  )}
                >
                  {e.key}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* When the container was created — the tree's footer styling. */}
      {createdAgo !== "" && (
        <span
          data-created-ago
          className={cn(
            "self-end",
            "text-xs",
            "text-info-mid",
            "tabular-nums",
          )}
        >
          created {createdAgo}
        </span>
      )}
    </div>
  );
}

function DetailRow({ label, value }: { label: string; value: string }) {
  if (value === "") return null;
  return (
    <div className={cn("flex", "gap-2")}>
      <span className={cn("shrink-0", "text-info-dim")}>{label}</span>
      <span className={cn("min-w-0", "break-all")}>{value}</span>
    </div>
  );
}
