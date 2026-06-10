import { useEffect, useState } from "react";
import { tauriListen } from "../lib/tauri";
import type { ViewerEvent } from "../types";

export interface ApiCallEntry {
  id: string;
  subType: string;
  method: string;
  path: string;
  status: "streaming" | "complete" | "error";
  startedAt: number;
  endedAt: number | null;
  chunkCount: number;
  error: unknown | null;
}

const MAX_ENTRIES = 100;

function parseSubType(subType: string): { method: string; path: string } {
  const idx = subType.indexOf("_");
  if (idx === -1) return { method: "", path: subType };
  return { method: subType.slice(0, idx), path: subType.slice(idx + 1) };
}

export function useApiCalls(options?: { disabled?: boolean }): ApiCallEntry[] {
  const [entries, setEntries] = useState<ApiCallEntry[]>([]);

  useEffect(() => {
    if (options?.disabled) return;

    const unlisten = tauriListen<ViewerEvent>("objectiveai", (event) => {
      const payload = event.payload;
      if (payload.type !== "api_call") return;

      const envelope = payload.value as { type: string; error?: unknown } | null;
      if (!envelope || typeof envelope !== "object") return;

      const subType = payload.sub_type;

      setEntries((prev) => {
        if (envelope.type === "begin") {
          const { method, path } = parseSubType(subType);
          const entry: ApiCallEntry = {
            id: `${subType}-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`,
            subType,
            method,
            path,
            status: "streaming",
            startedAt: Date.now(),
            endedAt: null,
            chunkCount: 0,
            error: null,
          };
          const next = [entry, ...prev];
          return next.length > MAX_ENTRIES ? next.slice(0, MAX_ENTRIES) : next;
        }

        const activeIdx = prev.findIndex(
          (e) => e.subType === subType && e.status === "streaming",
        );
        if (activeIdx === -1) return prev;

        if (envelope.type === "chunk") {
          const updated = { ...prev[activeIdx], chunkCount: prev[activeIdx].chunkCount + 1 };
          const next = [...prev];
          next[activeIdx] = updated;
          return next;
        }

        if (envelope.type === "error") {
          const updated = { ...prev[activeIdx], status: "error" as const, endedAt: Date.now(), error: envelope.error };
          const next = [...prev];
          next[activeIdx] = updated;
          return next;
        }

        if (envelope.type === "end") {
          const updated = { ...prev[activeIdx], status: "complete" as const, endedAt: Date.now() };
          const next = [...prev];
          next[activeIdx] = updated;
          return next;
        }

        return prev;
      });
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [options?.disabled]);

  return entries;
}
