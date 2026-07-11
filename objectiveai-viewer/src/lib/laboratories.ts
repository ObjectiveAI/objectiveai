import type { LaboratoryStatus } from "../hooks/useLaboratoriesList";
import type { ViewerLaboratory } from "../hooks/useViewerLaboratories";

/** The VIEWER's 3-way provenance — a refinement of the daemon's
 * 2-way `source` by one extra fact, the viewer machine's own scan:
 *
 * - `local`  — the id is in the VIEWER's scan (pure set membership;
 *              no machine-identity logic).
 * - `daemon` — not on the viewer machine, but local to the DAEMON's
 *              machine (its item says `source: "local"`). Empty by
 *              construction when viewer and daemon share a machine.
 * - `remote` — everything else.
 */
export type ViewerSource = "local" | "daemon" | "remote";

/** One laboratory as the laboratories page displays it. */
export interface DisplayLaboratory {
  id: string;
  image: string;
  mounts: { host: string; container: string }[];
  env: { key: string; value: string }[];
  cwd: string;
  source: ViewerSource;
  connected: boolean;
}

/**
 * The display union: every daemon-listed laboratory (reclassified by
 * viewer-scan membership) plus viewer-scan laboratories the daemon
 * doesn't know (created locally, never connected) — those are
 * `local`, not connected, identity from the scan. Sorted by id.
 */
export function mergeLaboratories(
  daemon: LaboratoryStatus[],
  viewer: ViewerLaboratory[],
): DisplayLaboratory[] {
  const viewerIds = new Set(viewer.map((lab) => lab.id));
  const daemonIds = new Set(daemon.map((lab) => lab.id));
  const out: DisplayLaboratory[] = [];
  for (const lab of daemon) {
    const source: ViewerSource = viewerIds.has(lab.id)
      ? "local"
      : lab.source === "local"
        ? "daemon"
        : "remote";
    out.push({
      id: lab.id,
      image: lab.image,
      mounts: lab.mounts,
      env: lab.env,
      cwd: lab.cwd,
      source,
      connected: lab.connected,
    });
  }
  for (const lab of viewer) {
    if (daemonIds.has(lab.id)) continue;
    out.push({
      id: lab.id,
      image: lab.image,
      mounts: lab.mounts,
      env: lab.env.map(([key, value]) => ({ key, value })),
      cwd: lab.cwd,
      source: "local",
      connected: false,
    });
  }
  return out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}
