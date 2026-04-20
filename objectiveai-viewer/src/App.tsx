import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { AgentCompletionView } from "./AgentCompletionView";
import { z } from "zod";
import {
  AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema,
  LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema,
  ErrorResponseErrorSchema,
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged,
} from "objectiveai";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
} from "objectiveai";

// Extended schemas with required id
const AgentCompletionCreateParamsSchema = AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
  id: z.string(),
});
type AgentCompletionCreateParams = z.infer<typeof AgentCompletionCreateParamsSchema>;

const FunctionExecutionCreateParamsSchema = FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema.extend({
  id: z.string(),
});
type FunctionExecutionCreateParams = z.infer<typeof FunctionExecutionCreateParamsSchema>;

const FunctionInventionRecursiveCreateParamsSchema = FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema.extend({
  id: z.string(),
});
type FunctionInventionRecursiveCreateParams = z.infer<typeof FunctionInventionRecursiveCreateParamsSchema>;

const LaboratoryExecutionCreateParamsSchema = LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema.extend({
  id: z.string(),
});
type LaboratoryExecutionCreateParams = z.infer<typeof LaboratoryExecutionCreateParamsSchema>;

const ResponseErrorSchema = ErrorResponseErrorSchema.extend({
  id: z.string(),
});
type ResponseError = z.infer<typeof ResponseErrorSchema>;

// Classified incoming event
type AgentCompletionEvent =
  | { type: "begin"; data: AgentCompletionCreateParams }
  | { type: "chunk"; data: AgentCompletionsResponseStreamingAgentCompletionChunk }
  | { type: "error"; data: ResponseError };

type FunctionExecutionEvent =
  | { type: "begin"; data: FunctionExecutionCreateParams }
  | { type: "chunk"; data: FunctionsExecutionsResponseStreamingFunctionExecutionChunk }
  | { type: "error"; data: ResponseError };

type FunctionInventionRecursiveEvent =
  | { type: "begin"; data: FunctionInventionRecursiveCreateParams }
  | { type: "chunk"; data: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk }
  | { type: "error"; data: ResponseError };

type LaboratoryExecutionEvent =
  | { type: "begin"; data: LaboratoryExecutionCreateParams }
  | { type: "chunk"; data: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk }
  | { type: "error"; data: ResponseError };

// Entry in the list
interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: AgentCompletionCreateParams;
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: ResponseError | null;
}

interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: FunctionExecutionCreateParams;
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: ResponseError | null;
}

interface FunctionInventionRecursiveEntry {
  kind: "invention";
  id: string;
  request: FunctionInventionRecursiveCreateParams;
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: ResponseError | null;
}

interface LaboratoryExecutionEntry {
  kind: "laboratory";
  id: string;
  request: LaboratoryExecutionCreateParams;
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk | null;
  error: ResponseError | null;
}

type Entry = AgentCompletionEntry | FunctionExecutionEntry | FunctionInventionRecursiveEntry | LaboratoryExecutionEntry;

function classifyAgentCompletion(payload: unknown): AgentCompletionEvent | null {
  const beginParse = AgentCompletionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyFunctionExecution(payload: unknown): FunctionExecutionEvent | null {
  const beginParse = FunctionExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyFunctionInventionRecursive(payload: unknown): FunctionInventionRecursiveEvent | null {
  const beginParse = FunctionInventionRecursiveCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyLaboratoryExecution(payload: unknown): LaboratoryExecutionEvent | null {
  const beginParse = LaboratoryExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function EntryView({ entry }: { entry: Entry }) {
  if (entry.kind === "agent-completion") {
    return <AgentCompletionView entry={entry} />;
  }
  if (entry.error) {
    return <pre style={{ color: "red" }}>{JSON.stringify(entry.error, null, 2)}</pre>;
  }
  if (entry.chunk) {
    return <pre>{JSON.stringify(entry.chunk, null, 2)}</pre>;
  }
  return <pre style={{ color: "gray" }}>{JSON.stringify(entry.request, null, 2)}</pre>;
}

function App() {
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
              kind: "execution",
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
              kind: "invention",
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

    return () => {
      unlistenAgentCompletion.then((fn) => fn());
      unlistenExecution.then((fn) => fn());
      unlistenInvention.then((fn) => fn());
      unlistenLaboratory.then((fn) => fn());
    };
  }, []);

  return (
    <main className="container">
      <h1>ObjectiveAI Viewer</h1>
      {entries.length === 0 && <p>Waiting for requests...</p>}
      {entries.map((entry) => (
        <EntryView key={entry.id} entry={entry} />
      ))}
    </main>
  );
}

export default App;
