import { useEffect, useState } from "react";
import {
  Client,
  LaboratoriesListListener,
  type DaemonLaboratoriesListListenerLaboratoryStatus,
  type LaboratoriesInlineLaboratoryImage,
  type LaboratoriesLaboratoryImage,
  type ViewerTransport,
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
  DaemonLaboratoriesListListenerLaboratoryStatus;

/**
 * The daemon's live laboratories list (`/laboratories/list`) as a
 * stream. VISIBILITY FOLLOWS THE HOST: the list contains only
 * laboratories served by a CONNECTED laboratory host, and hosts spawn
 * lazily (the first `laboratories create`/`attach` spawns the local
 * one) — so an empty list on a fresh daemon does NOT mean no
 * containers exist; pre-existing ones appear all at once when
 * something spawns the host and it announces podman's current set.
 * Mirrors [`useAgentsInstancesList`]: one listener per hook instance,
 * 1s reconnect loop, errors to the toast. `null` transport yields an
 * empty list.
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
          const listener = await Client.viewer(
            transport,
          ).laboratoriesListListener();
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          setLaboratories(listener.laboratories());
          while (!listener.closed) {
            await listener.subscribe();
            if (cancelled) return;
            setLaboratories(listener.laboratories());
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
