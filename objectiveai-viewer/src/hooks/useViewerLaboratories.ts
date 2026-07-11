import { useEffect, useState } from "react";
import { tauriInvoke } from "../lib/tauri";
import { reportError } from "../lib/errors";

/** One laboratory from the VIEWER machine's own podman scan — the
 * `laboratories_list` Tauri command's `Identify` JSON. Deliberately a
 * local interface, not an SDK type: `Identify` is daemon↔manager wire,
 * exempt from the public schema surface. */
export interface ViewerLaboratory {
  id: string;
  image: string;
  mounts: { host: string; container: string }[];
  env: [string, string][];
  cwd: string;
}

/** Rescan cadence while the laboratories tab is focused. */
const SCAN_INTERVAL_MS = 30_000;

/**
 * The viewer machine's laboratories (running or not), via the
 * `laboratories_list` Tauri command (a podman label scan by the local
 * `objectiveai-laboratory` binary). Fetches once on mount and then
 * every 30s WHILE `active` — the pane stays mounted across tab swaps,
 * so polling is gated on the tab actually being focused. A failed
 * scan reports to the toast and keeps the last good result.
 */
export function useViewerLaboratories(active: boolean): ViewerLaboratory[] {
  const [laboratories, setLaboratories] = useState<ViewerLaboratory[]>([]);

  useEffect(() => {
    let cancelled = false;
    const scan = async () => {
      try {
        const result = await tauriInvoke<ViewerLaboratory[]>(
          "laboratories_list",
        );
        if (!cancelled && result) setLaboratories(result);
      } catch (error) {
        reportError("viewer laboratories", error);
      }
    };
    // One scan on mount regardless — the first classification
    // shouldn't wait for a tab focus.
    void scan();
    if (!active) return () => {
      cancelled = true;
    };
    const interval = setInterval(() => void scan(), SCAN_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [active]);

  return laboratories;
}
