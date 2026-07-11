import { useEffect, useState } from "react";
import {
  WebSocketLaboratoriesListListener,
  type CliWebsocketLaboratoriesListListenerLaboratoryStatus,
} from "@objectiveai/sdk";
import type { DaemonConnection } from "../lib/daemon";
import { reportError } from "../lib/errors";

/** One laboratory on the daemon's live list — the SDK wire type,
 * re-exported under the viewer's short name. */
export type LaboratoryStatus =
  CliWebsocketLaboratoriesListListenerLaboratoryStatus;

/**
 * The daemon's live laboratories list (`/laboratories/list`): the
 * `laboratories list` merge as a stream — connected ∪ daemon-local
 * scan, each with `source` (the DAEMON's local/remote vantage) and a
 * live `connected` flag. Mirrors [`useAgentsInstancesList`]: one
 * listener per hook instance, 1s reconnect loop, errors to the toast.
 * `null` connection yields an empty list.
 */
export function useLaboratoriesList(
  connection: DaemonConnection | null,
): LaboratoryStatus[] {
  const [laboratories, setLaboratories] = useState<LaboratoryStatus[]>([]);

  useEffect(() => {
    setLaboratories([]);
    if (connection === null) return;
    let cancelled = false;
    let current: WebSocketLaboratoriesListListener | null = null;

    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await WebSocketLaboratoriesListListener.connect(
            `${connection.address}/laboratories/list`,
            {
              signature: connection.signature,
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
  }, [connection]);

  return laboratories;
}
