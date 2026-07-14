/**
 * One laboratory's live file tree from the daemon's
 * `/laboratories/{id}/filetree` endpoint, materialized. `null` until
 * the first frame (connecting); an EMPTY array is a real state — the
 * container hasn't started or has no files (files appear when the
 * lazily-started container runs). Mirrors [`useLaboratoriesList`]:
 * one listener per hook instance, 1s reconnect loop, errors to the
 * toast. `null` transport yields `null`.
 *
 * The listener's fold is path-copying immutable: each change produces
 * a NEW root array whose unchanged subtrees keep their identity — set
 * directly into state, so memoized tree nodes re-render only in the
 * changed region.
 */
import { useEffect, useState } from "react";
import {
  LaboratoriesFiletreeListener,
  type LaboratoriesFiletreeFileTreeNode,
  type ViewerTransport,
} from "@objectiveai/sdk";
import { reportError } from "../lib/errors";

export type FileTreeNode = LaboratoriesFiletreeFileTreeNode;

export function useLaboratoryFiletree(
  transport: ViewerTransport | null,
  id: string,
  machine?: string,
  machineState?: string,
): FileTreeNode[] | null {
  const [children, setChildren] = useState<FileTreeNode[] | null>(null);

  useEffect(() => {
    setChildren(null);
    if (transport === null) return;
    let cancelled = false;
    let current: LaboratoriesFiletreeListener | null = null;

    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await LaboratoriesFiletreeListener.connectViewer(
            transport,
            id,
            {
              machine,
              machineState,
              onChange: (next) => {
                if (!cancelled) setChildren(next);
              },
            },
          );
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          setChildren(listener.children());
          // Ride the connection until it closes (subscribe resolves on
          // every change AND on close).
          while (!listener.closed) {
            await listener.subscribe();
          }
        } catch (error) {
          // Connect refused / handshake failure — surface it, then retry.
          reportError("laboratory filetree", error);
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
  }, [transport, id, machine, machineState]);

  return children;
}
