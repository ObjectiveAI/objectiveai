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
  /** Unix seconds when the container was created — podman's own
   * record; absent from scans by older laboratory binaries. */
  created_at?: number | null;
}

/**
 * The viewer machine's laboratories (running or not), via the
 * `laboratories_list` Tauri command (a podman label scan by the local
 * `objectiveai-laboratory` binary). Scans exactly ONCE per effect run
 * — on mount and again only when `active` transitions (tab focus).
 * NO timer: the podman subprocess never fires on an interval. A
 * failed scan reports to the toast and keeps the last good result.
 */
export function useViewerLaboratories(active: boolean): ViewerLaboratory[] {
  const [laboratories, setLaboratories] = useState<ViewerLaboratory[]>([]);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const result = await tauriInvoke<ViewerLaboratory[]>(
          "laboratories_list",
        );
        if (!cancelled && result) setLaboratories(result);
      } catch (error) {
        reportError("viewer laboratories", error);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [active]);

  return laboratories;
}
