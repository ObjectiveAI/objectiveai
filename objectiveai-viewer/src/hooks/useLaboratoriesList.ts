import { useEffect, useState } from "react";
import {
  LaboratoriesListListener,
  type ViewerTransport,
  type CliLaboratoriesListListenerLaboratoryStatus,
  type LaboratoriesInlineLaboratoryImage,
  type LaboratoriesLaboratoryImage,
} from "@objectiveai/sdk";
import { reportError } from "../lib/errors";

/** A laboratory's base image — the SDK wire type (an INLINE
 * Containerfile spec, or a split registry reference), re-exported
 * under the viewer's short name. */
export type LaboratoryImage = LaboratoriesLaboratoryImage;

/** Narrow a LaboratoryImage to its inline form. */
export function isInlineImage(
  image: LaboratoryImage,
): image is LaboratoriesInlineLaboratoryImage {
  return "containerfile" in image;
}

/** One laboratory on the daemon's live list — the SDK wire type,
 * re-exported under the viewer's short name. */
export type LaboratoryStatus =
  CliLaboratoriesListListenerLaboratoryStatus;

/**
 * The daemon's live laboratories list (`/laboratories/list`): the
 * `laboratories list` merge as a stream — connected ∪ daemon-local
 * scan, each with `source` (the DAEMON's local/remote vantage) and a
 * live `connected` flag. Mirrors [`useAgentsInstancesList`]: one
 * listener per hook instance, 1s reconnect loop, errors to the toast.
 * `null` transport yields an empty list.
 */
export function useLaboratoriesList(
  transport: ViewerTransport | null,
): LaboratoryStatus[] {
  const [laboratories, setLaboratories] = useState<LaboratoryStatus[]>([]);

  useEffect(() => {
    setLaboratories([]);
    if (transport === null) return;
    let cancelled = false;
    let current: LaboratoriesListListener | null = null;

    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await LaboratoriesListListener.connectViewer(
            transport,
            {
              onChange: (next) => {
                if (!cancelled) setLaboratories(next);
              },
            },
          );
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          setLaboratories(listener.laboratories());
          while (!listener.closed) {
            await listener.subscribe();
          }
        } catch (error) {
          reportError("laboratories list", error);
        }
        current = null;
        if (cancelled) return;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    })();

    return () => {
      cancelled = true;
      current?.close();
    };
  }, [transport]);

  return laboratories;
}
