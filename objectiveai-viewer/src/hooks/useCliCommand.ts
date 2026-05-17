import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { ViewerEvent } from "../types";

export interface CliLine {
  type: "begin" | "chunk" | "error" | "end";
  value: unknown;
}

export function useCliCommand() {
  const [lines, setLines] = useState<CliLine[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const unlisten = listen<ViewerEvent>("objectiveai", (event) => {
      const payload = event.payload;
      if (payload.type !== "cli_command") return;

      const envelope = payload.value as { type: string; chunk?: unknown; error?: unknown } | null;
      if (!envelope || typeof envelope !== "object") return;

      setLines((prev) => [...prev, { type: envelope.type as CliLine["type"], value: envelope.chunk ?? envelope.error ?? null }]);

      if (envelope.type === "end" || envelope.type === "error") {
        setIsRunning(false);
      }
    });

    unlisten.then((fn) => {
      unlistenRef.current = fn;
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const run = useCallback((args: string[]) => {
    setLines([]);
    setIsRunning(true);
    invoke("cli_run", { args, origin: "objectiveai" }).catch((e) => {
      setLines((prev) => [...prev, { type: "error", value: String(e) }]);
      setIsRunning(false);
    });
  }, []);

  const clear = useCallback(() => {
    setLines([]);
  }, []);

  return { lines, isRunning, run, clear };
}
