import type { LaboratoryStatus } from "../hooks/useLaboratoriesList";
import type { MachineIdentity } from "../hooks/useMachineIdentity";

/** The VIEWER's 2-way provenance, by MACHINE IDENTITY (there is no
 * scan and no daemon-side local/remote — machine identity is the only
 * provenance):
 *
 * - `local`  — the laboratory's serving host runs on THIS machine
 *              (`machine.id` equals the viewer's own machine id).
 * - `remote` — everything else, including items whose machine is
 *              unknown (identity unresolved ⇒ never guess local).
 */
export type ViewerSource = "local" | "remote";

/** One laboratory as the laboratories page displays it. */
export interface DisplayLaboratory {
  id: string;
  image: string;
  mounts: { host: string; container: string }[];
  env: { key: string; value: string }[];
  cwd: string;
  /** Unix seconds when the container was created; `null` when the
   * source didn't report it. */
  createdAt: number | null;
  source: ViewerSource;
  /** The machine whose host serves this laboratory — display
   * metadata (hostname, os) for the card; `null` when unreported. */
  machine: MachineIdentity | null;
  connected: boolean;
}

/**
 * The display list: every daemon-listed laboratory, classified by
 * comparing its serving host's `machine.id` against the viewer's own
 * machine identity. The registry is the whole universe — hosts
 * announce their full set and notify on every change, so there are no
 * scan-only extras to merge. Sorted by id.
 */
export function classifyLaboratories(
  daemon: LaboratoryStatus[],
  machine: MachineIdentity | null,
): DisplayLaboratory[] {
  const out: DisplayLaboratory[] = daemon.map((lab) => ({
    id: lab.id,
    image: lab.image,
    mounts: lab.mounts,
    env: lab.env,
    cwd: lab.cwd,
    createdAt: lab.created_at ?? null,
    source:
      machine !== null && lab.machine?.id === machine.id
        ? "local"
        : "remote",
    machine: lab.machine ?? null,
    connected: lab.connected,
  }));
  return out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}
