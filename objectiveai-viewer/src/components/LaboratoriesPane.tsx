import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import { useAgo } from "../hooks/useAgo";
import { isInlineImage, useLaboratoriesList } from "../hooks/useLaboratoriesList";
import {
  classifyLaboratories,
  type DisplayLaboratory,
} from "../lib/laboratories";
import { builtinTabModule, tabsOpen } from "../lib/tabs";
import { LogoMark } from "./shared/Logo";
import { OpenTab } from "./shared/OpenTab";

/**
 * The `laboratories` home pane: the daemon's `/laboratories/list`
 * stream (the host registry — the whole laboratory universe). Each
 * card shows the laboratory's spec plus its serving host's machine
 * identity verbatim (os, hostname, machine id, state) — machine
 * identity is the only provenance; there is no local/remote
 * classification. All laboratory state lives HERE — unshared with
 * the agents tab.
 */
export function LaboratoriesPane({
  transport,
}: {
  transport: ViewerTransport | null;
}) {
  const daemon = useLaboratoriesList(transport);
  const laboratories = classifyLaboratories(daemon);

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
          <LaboratoryCard
            key={`${lab.machine?.id ?? ""}\n${lab.machineState ?? ""}\n${lab.id}`}
            lab={lab}
          />
        ))}
      </div>
    </div>
  );
}

/** Open (or focus) the laboratory's filesystem TAB — appended to this
 * window's strip, or focused wherever it already lives (the shell
 * dedupes by kind). The `{os}/{machine}/{id}` title format is the
 * old bespoke window title's. */
function openLaboratoryWindow(lab: DisplayLaboratory): void {
  void tabsOpen({
    module: builtinTabModule("laboratory"),
    title: `${lab.machine?.os ?? "?"}/${lab.machine?.id ?? "?"}/${lab.id}`,
    arguments: {
      id: lab.id,
      ...(lab.machine?.id !== undefined ? { machine: lab.machine.id } : {}),
      ...(lab.machineState !== undefined && lab.machineState !== null
        ? { machine_state: lab.machineState }
        : {}),
    },
  });
}

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
      {/* Header: running dot + id + the open tab (the top-right
          corner belongs to the opener, same as the agent box). The
          dot is the CONTAINER's live run state — `connected` is
          tautologically true here (the list only carries labs on
          connected hosts). */}
      <div className={cn("flex", "items-center", "gap-2")}>
        <span
          data-running={lab.running}
          className={cn(
            "w-1.5",
            "h-1.5",
            "rounded-full",
            "shrink-0",
            lab.running
              ? cn("bg-copper-hot", "animate-pulse")
              : "bg-info-dim",
          )}
          title={lab.running ? "running" : "stopped"}
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
        <OpenTab
          dataAttr="data-open-laboratory"
          onClick={() => openLaboratoryWindow(lab)}
          ariaLabel={`Open ${lab.id} filesystem`}
          className={cn("self-start", "-mt-[9px]", "-mr-[11px]")}
        />
      </div>

      {/* Spec detail — the machine identity first: where this
          laboratory belongs. */}
      <div className={cn("flex", "flex-col", "gap-1", "text-xs", "text-[#c3bfbb]")}>
        {lab.machine === null ? (
          <DetailRow label="machine" value="unknown" />
        ) : (
          <>
            <DetailRow label="os" value={lab.machine.os} />
            <DetailRow label="hostname" value={lab.machine.hostname ?? ""} />
            {/* The FULL id — DetailRow wraps (break-all) if the card
                genuinely runs out of room; no pre-truncation. */}
            <DetailRow label="machine" value={lab.machine.id} />
            <DetailRow label="state" value={lab.machineState ?? ""} />
          </>
        )}
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

function DetailRow({
  label,
  value,
  title,
}: {
  label: string;
  value: string;
  title?: string;
}) {
  if (value === "") return null;
  return (
    <div className={cn("flex", "gap-2")}>
      <span className={cn("shrink-0", "text-info-dim")}>{label}</span>
      <span className={cn("min-w-0", "break-all")} title={title}>
        {value}
      </span>
    </div>
  );
}
