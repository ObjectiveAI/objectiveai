import { useEffect, useState } from "react";
import { tauriInvoke } from "../lib/tauri";
import { reportError } from "../lib/errors";

/** THIS machine's identity from the `machine_identity` Tauri command —
 * the SDK's `machine.MachineIdentity` JSON. The stable hashed `id` is
 * what laboratory items' `machine.id` is compared against; `os` /
 * `hostname` are display metadata. */
export interface MachineIdentity {
  id: string;
  os: string;
  hostname?: string | null;
}

/**
 * The viewer machine's identity, fetched exactly ONCE per mount —
 * machine identity never changes while the process runs. `null` until
 * resolved (or when the Tauri command is unavailable — classification
 * then degrades to "remote" everywhere rather than guessing).
 */
export function useMachineIdentity(): MachineIdentity | null {
  const [machine, setMachine] = useState<MachineIdentity | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const result = await tauriInvoke<MachineIdentity>("machine_identity");
        if (!cancelled && result) setMachine(result);
      } catch (error) {
        reportError("machine identity", error);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return machine;
}
