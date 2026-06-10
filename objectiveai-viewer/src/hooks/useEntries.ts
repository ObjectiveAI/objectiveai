import { useEffect, useState } from "react";
import { tauriListen, tauriInvoke } from "../lib/tauri";
import {
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged,
} from "@objectiveai/sdk";
import type { Entry, ViewerEvent } from "../types";
import {
  classifyAgentCompletion,
  classifyFunctionExecution,
  classifyFunctionInventionRecursive,
  classifyLaboratoryExecution,
} from "../classify";

interface ClassifiedBegin { type: "begin"; data: { id: string } }
interface ClassifiedError { type: "error"; data: { id: string; code: number; message: unknown } }
interface ClassifiedChunk { type: "chunk"; data: unknown }
type ClassifyResult = ClassifiedBegin | ClassifiedError | ClassifiedChunk;

interface SubTypeConfig {
  kind: Entry["kind"];
  classify: (payload: unknown) => ClassifyResult | null;
  merge: (a: unknown, b: unknown) => [unknown, boolean];
}

const SUB_TYPE_MAP: Record<string, SubTypeConfig> = {
  agent_completions: {
    kind: "agent-completion",
    classify: classifyAgentCompletion,
    merge: (a, b) => agentCompletionsResponseStreamingAgentCompletionChunkMerged(a as never, b as never) as [unknown, boolean],
  },
  functions_executions: {
    kind: "execution",
    classify: classifyFunctionExecution,
    merge: (a, b) => functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(a as never, b as never) as [unknown, boolean],
  },
  functions_inventions_recursive: {
    kind: "invention",
    classify: classifyFunctionInventionRecursive,
    merge: (a, b) => functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(a as never, b as never) as [unknown, boolean],
  },
  laboratories_executions: {
    kind: "laboratory",
    classify: classifyLaboratoryExecution,
    merge: (a, b) => laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(a as never, b as never) as [unknown, boolean],
  },
};

function handleEvent(
  payload: ViewerEvent,
  setEntries: React.Dispatch<React.SetStateAction<Entry[]>>,
): void {
  if (payload.type !== "inbound") return;

  const config = SUB_TYPE_MAP[payload.sub_type];
  if (!config) return;

  const classified = config.classify(payload.value);
  if (!classified) return;

  setEntries((prev) => {
    if (classified.type === "begin") {
      return [...prev, {
        kind: config.kind,
        id: classified.data.id,
        receivedAt: Date.now(),
        request: classified.data,
        chunk: null,
        error: null,
      } as Entry];
    }
    if (classified.type === "error") {
      const id = classified.data.id;
      if (!prev.some((e) => e.id === id && e.kind === config.kind)) return prev;
      return prev.map((e) =>
        e.id === id && e.kind === config.kind ? { ...e, error: classified.data } as Entry : e
      );
    }
    const id = (classified.data as { id: string }).id;
    if (!prev.some((e) => e.id === id && e.kind === config.kind)) return prev;
    return prev.map((e) => {
      if (e.id !== id || e.kind !== config.kind) return e;
      const [merged] = e.chunk
        ? config.merge(e.chunk, classified.data)
        : [classified.data, true];
      return { ...e, chunk: merged } as Entry;
    });
  });
}

export function useEntries(options?: {
  disabled?: boolean;
  initialEntries?: Entry[] | null;
}): Entry[] {
  const [entries, setEntries] = useState<Entry[]>([]);

  useEffect(() => {
    if (options?.initialEntries) {
      setEntries(options.initialEntries);
    }
  }, [options?.initialEntries]);

  useEffect(() => {
    if (options?.disabled) return;

    let cancelled = false;

    const unlisten = tauriListen<ViewerEvent>("objectiveai", (event) => {
      handleEvent(event.payload, setEntries);
    });

    unlisten.then(() => {
      if (!cancelled) tauriInvoke("viewer_ready");
    });

    return () => {
      cancelled = true;
      unlisten.then((fn) => fn());
    };
  }, [options?.disabled]);

  return entries;
}
