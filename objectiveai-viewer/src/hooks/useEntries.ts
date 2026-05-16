import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged,
} from "@objectiveai/sdk";
import type { Entry, ResponseError } from "../types";
import {
  classifyAgentCompletion,
  classifyFunctionExecution,
  classifyFunctionInventionRecursive,
  classifyLaboratoryExecution,
} from "../classify";

interface ClassifiedBegin { type: "begin"; data: { id: string } }
interface ClassifiedError { type: "error"; data: ResponseError }
interface ClassifiedChunk { type: "chunk"; data: unknown }
type ClassifyResult = ClassifiedBegin | ClassifiedError | ClassifiedChunk;

function createListener(
  channel: string,
  kind: Entry["kind"],
  classify: (payload: unknown) => ClassifyResult | null,
  merge: (a: unknown, b: unknown) => [unknown, boolean],
  setEntries: React.Dispatch<React.SetStateAction<Entry[]>>,
): Promise<() => void> {
  return listen<unknown>(channel, (event) => {
    const classified = classify(event.payload);
    if (!classified) return;

    setEntries((prev) => {
      if (classified.type === "begin") {
        return [...prev, {
          kind,
          id: classified.data.id,
          request: classified.data,
          chunk: null,
          error: null,
        } as Entry];
      }
      if (classified.type === "error") {
        const id = classified.data.id;
        if (!prev.some((e) => e.id === id && e.kind === kind)) return prev;
        return prev.map((e) =>
          e.id === id && e.kind === kind ? { ...e, error: classified.data } as Entry : e
        );
      }
      const id = (classified.data as { id: string }).id;
      if (!prev.some((e) => e.id === id && e.kind === kind)) return prev;
      return prev.map((e) => {
        if (e.id !== id || e.kind !== kind) return e;
        const [merged] = e.chunk
          ? merge(e.chunk, classified.data)
          : [classified.data, true];
        return { ...e, chunk: merged } as Entry;
      });
    });
  });
}

export function useEntries(): Entry[] {
  const [entries, setEntries] = useState<Entry[]>([]);

  useEffect(() => {
    let cancelled = false;

    const wrap = <T,>(fn: (a: T, b: T) => [T, boolean]) =>
      (a: unknown, b: unknown) => fn(a as T, b as T) as [unknown, boolean];

    const listeners = [
      createListener("agent-completions", "agent-completion", classifyAgentCompletion, wrap(agentCompletionsResponseStreamingAgentCompletionChunkMerged), setEntries),
      createListener("functions-executions", "execution", classifyFunctionExecution, wrap(functionsExecutionsResponseStreamingFunctionExecutionChunkMerged), setEntries),
      createListener("functions-inventions-recursive", "invention", classifyFunctionInventionRecursive, wrap(functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged), setEntries),
      createListener("laboratories-executions", "laboratory", classifyLaboratoryExecution, wrap(laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged), setEntries),
    ];

    Promise.all(listeners).then(() => {
      if (!cancelled) invoke("viewer_ready");
    });

    return () => {
      cancelled = true;
      for (const l of listeners) l.then((fn) => fn());
    };
  }, []);

  return entries;
}
