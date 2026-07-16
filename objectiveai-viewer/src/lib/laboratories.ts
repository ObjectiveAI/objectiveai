import type { LaboratoryImage, LaboratoryStatus } from "../hooks/useLaboratoriesList";
import type { MachineIdentity } from "../hooks/useMachineIdentity";

/** One laboratory as the laboratories page displays it. */
export interface DisplayLaboratory {
  id: string;
  image: LaboratoryImage;
  mounts: { host: string; container: string }[];
  env: { key: string; value: string }[];
  cwd: string;
  /** Unix seconds when the container was created; `null` when the
   * source didn't report it. */
  createdAt: number | null;
  /** For agent laboratories, the full id of the source agent; `null`
   * for user-created laboratories. */
  agentFullId: string | null;
  /** The machine whose host serves this laboratory — the card's
   * provenance display (os, hostname, id); `null` when unreported. */
  machine: MachineIdentity | null;
  /** The serving host's state name — pins the id together with
   * `machine` (laboratory ids are only unique per (machine, state)). */
  machineState: string | null;
  connected: boolean;
  /** Whether the laboratory's CONTAINER is running right now — the
   * lifecycle starts/stops containers on demand, and every transition
   * streams as an upsert, so this is live state. */
  running: boolean;
}

/** The display list: every daemon-listed laboratory, sorted by id.
 * Machine identity is the only provenance — the card shows it
 * verbatim (no local/remote classification). */
export function classifyLaboratories(
  daemon: LaboratoryStatus[],
): DisplayLaboratory[] {
  const out: DisplayLaboratory[] = daemon.map((lab) => ({
    id: lab.id,
    image: lab.image,
    mounts: lab.mounts,
    env: lab.env,
    cwd: lab.cwd,
    createdAt: lab.created_at ?? null,
    agentFullId: lab.agent_full_id ?? null,
    machine: lab.machine ?? null,
    machineState: lab.machine_state ?? null,
    connected: lab.connected,
    running: lab.running ?? false,
  }));
  return out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}
