import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged,
} from "objectiveai";
import type { Entry } from "../types";
import {
  classifyAgentCompletion,
  classifyFunctionExecution,
  classifyFunctionInventionRecursive,
  classifyLaboratoryExecution,
} from "../classify";

export function useEntries(): Entry[] {
  const [entries, setEntries] = useState<Entry[]>([]);

  useEffect(() => {
    const unlistenAgentCompletion = listen<unknown>("agent-completions", (event) => {
      const classified = classifyAgentCompletion(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "agent-completion" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "agent-completion")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "agent-completion") return e;
              const [merged] = e.chunk
                ? agentCompletionsResponseStreamingAgentCompletionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenExecution = listen<unknown>("functions-executions", (event) => {
      const classified = classifyFunctionExecution(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "execution" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "execution")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "execution") return e;
              const [merged] = e.chunk
                ? functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenInvention = listen<unknown>("functions-inventions-recursive", (event) => {
      const classified = classifyFunctionInventionRecursive(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "invention" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "invention")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "invention") return e;
              const [merged] = e.chunk
                ? functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenLaboratory = listen<unknown>("laboratories-executions", (event) => {
      const classified = classifyLaboratoryExecution(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "laboratory" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "laboratory")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "laboratory") return e;
              const [merged] = e.chunk
                ? laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    Promise.all([unlistenAgentCompletion, unlistenExecution, unlistenInvention, unlistenLaboratory])
      .then(() => invoke("viewer_ready"));

    return () => {
      unlistenAgentCompletion.then((fn) => fn());
      unlistenExecution.then((fn) => fn());
      unlistenInvention.then((fn) => fn());
      unlistenLaboratory.then((fn) => fn());
    };
  }, []);

  return entries;
}
