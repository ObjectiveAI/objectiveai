"use client";

import { useState, useEffect, useRef, useCallback } from "react";
import {
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

/* ── Entry types ── */

export interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: Record<string, unknown>;
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: { code: number; message: unknown } | null;
  timestamp: number;
}

export interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: Record<string, unknown>;
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: { code: number; message: unknown } | null;
  timestamp: number;
}

export interface FunctionInventionEntry {
  kind: "invention";
  id: string;
  request: Record<string, unknown>;
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: { code: number; message: unknown } | null;
  timestamp: number;
}

export interface LaboratoryExecutionEntry {
  kind: "laboratory";
  id: string;
  request: Record<string, unknown>;
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk | null;
  error: { code: number; message: unknown } | null;
  timestamp: number;
}

export type ViewerEntry =
  | AgentCompletionEntry
  | FunctionExecutionEntry
  | FunctionInventionEntry
  | LaboratoryExecutionEntry;

/* ── Event classification ── */

interface SSEPayload {
  kind: string;
  payload: Record<string, unknown>;
  timestamp: number;
}

function hasId(p: unknown): p is { id: string } {
  return typeof p === "object" && p !== null && "id" in p && typeof (p as Record<string, unknown>).id === "string";
}

function isError(p: unknown): p is { id: string; code: number; message: unknown } {
  return hasId(p) && typeof (p as Record<string, unknown>).code === "number";
}

function isBegin(p: unknown, objectPrefix?: string): boolean {
  if (!hasId(p)) return false;
  if (isError(p)) return false;
  const obj = p as Record<string, unknown>;
  if (obj.object && typeof obj.object === "string") {
    return !(obj.object as string).includes("chunk");
  }
  if ("messages" in obj || "function" in obj || "input" in obj || "name" in obj || "laboratory" in obj) return true;
  if (!obj.object) return true;
  return false;
}

function isChunk(p: unknown): boolean {
  if (!hasId(p)) return true;
  if (isError(p)) return false;
  const obj = p as Record<string, unknown>;
  if (obj.object && typeof obj.object === "string" && (obj.object as string).includes("chunk")) return true;
  if ("choices" in obj || "tasks" in obj || "inventions" in obj || "votes" in obj) return true;
  return false;
}

type Classified =
  | { type: "begin"; id: string; data: Record<string, unknown> }
  | { type: "chunk"; id: string; data: Record<string, unknown> }
  | { type: "error"; id: string; data: { code: number; message: unknown } };

function classify(payload: unknown): Classified | null {
  if (!payload || typeof payload !== "object") return null;
  const p = payload as Record<string, unknown>;

  if (isError(p)) {
    return { type: "error", id: (p as { id: string }).id, data: p as { code: number; message: unknown } };
  }

  if (hasId(p) && isBegin(p)) {
    return { type: "begin", id: (p as { id: string }).id, data: p };
  }

  const id = hasId(p) ? (p as { id: string }).id : null;
  if (id && isChunk(p)) {
    return { type: "chunk", id, data: p };
  }

  if (hasId(p)) {
    return { type: "chunk", id: (p as { id: string }).id, data: p };
  }

  return null;
}

/* ── Hook ── */

export type ConnectionState = "disconnected" | "connecting" | "connected" | "error";

export interface UseViewerEventsResult {
  entries: ViewerEntry[];
  connectionState: ConnectionState;
  clear: () => void;
}

export function useViewerEvents(url: string): UseViewerEventsResult {
  const [entries, setEntries] = useState<ViewerEntry[]>([]);
  const [connectionState, setConnectionState] = useState<ConnectionState>("disconnected");
  const sourceRef = useRef<EventSource | null>(null);

  const clear = useCallback(() => setEntries([]), []);

  useEffect(() => {
    setConnectionState("connecting");
    const source = new EventSource(url);
    sourceRef.current = source;

    source.onopen = () => setConnectionState("connected");

    source.onerror = () => {
      setConnectionState("error");
    };

    source.onmessage = (event) => {
      setConnectionState("connected");
      let parsed: SSEPayload;
      try {
        parsed = JSON.parse(event.data);
      } catch {
        return;
      }

      const classified = classify(parsed.payload);
      if (!classified) return;

      const { kind } = parsed;
      const ts = parsed.timestamp;

      setEntries((prev) => {
        switch (kind) {
          case "agent-completions":
            return handleAgentCompletion(prev, classified, ts);
          case "functions-executions":
            return handleFunctionExecution(prev, classified, ts);
          case "functions-inventions-recursive":
            return handleFunctionInvention(prev, classified, ts);
          case "laboratories-executions":
            return handleLaboratoryExecution(prev, classified, ts);
          default:
            return prev;
        }
      });
    };

    return () => {
      source.close();
      sourceRef.current = null;
    };
  }, [url]);

  return { entries, connectionState, clear };
}

/* ── Safe merge — SDK merge functions throw on malformed data ── */

function safeMerge<T>(prev: T, next: T, mergeFn: (a: T, b: T) => [T, boolean]): T {
  try {
    return mergeFn(prev, next)[0];
  } catch {
    return next;
  }
}

/* ── Per-kind handlers ── */

function handleAgentCompletion(
  prev: ViewerEntry[],
  classified: Classified,
  ts: number,
): ViewerEntry[] {
  switch (classified.type) {
    case "begin":
      return [
        ...prev,
        {
          kind: "agent-completion",
          id: classified.id,
          request: classified.data,
          chunk: null,
          error: null,
          timestamp: ts,
        },
      ];
    case "error": {
      if (!prev.some((e) => e.id === classified.id)) return prev;
      return prev.map((e) =>
        e.id === classified.id
          ? { ...e, error: classified.data }
          : e,
      );
    }
    case "chunk": {
      if (!prev.some((e) => e.id === classified.id && e.kind === "agent-completion"))
        return prev;
      return prev.map((e) => {
        if (e.id !== classified.id || e.kind !== "agent-completion") return e;
        const next = classified.data as AgentCompletionsResponseStreamingAgentCompletionChunk;
        const merged = e.chunk
          ? safeMerge(e.chunk, next, agentCompletionsResponseStreamingAgentCompletionChunkMerged)
          : next;
        return { ...e, chunk: merged };
      });
    }
  }
}

function handleFunctionExecution(
  prev: ViewerEntry[],
  classified: Classified,
  ts: number,
): ViewerEntry[] {
  switch (classified.type) {
    case "begin":
      return [
        ...prev,
        {
          kind: "execution",
          id: classified.id,
          request: classified.data,
          chunk: null,
          error: null,
          timestamp: ts,
        },
      ];
    case "error": {
      if (!prev.some((e) => e.id === classified.id)) return prev;
      return prev.map((e) =>
        e.id === classified.id ? { ...e, error: classified.data } : e,
      );
    }
    case "chunk": {
      if (!prev.some((e) => e.id === classified.id && e.kind === "execution"))
        return prev;
      return prev.map((e) => {
        if (e.id !== classified.id || e.kind !== "execution") return e;
        const next = classified.data as FunctionsExecutionsResponseStreamingFunctionExecutionChunk;
        const merged = e.chunk
          ? safeMerge(e.chunk, next, functionsExecutionsResponseStreamingFunctionExecutionChunkMerged)
          : next;
        return { ...e, chunk: merged };
      });
    }
  }
}

function handleFunctionInvention(
  prev: ViewerEntry[],
  classified: Classified,
  ts: number,
): ViewerEntry[] {
  switch (classified.type) {
    case "begin":
      return [
        ...prev,
        {
          kind: "invention",
          id: classified.id,
          request: classified.data,
          chunk: null,
          error: null,
          timestamp: ts,
        },
      ];
    case "error": {
      if (!prev.some((e) => e.id === classified.id)) return prev;
      return prev.map((e) =>
        e.id === classified.id ? { ...e, error: classified.data } : e,
      );
    }
    case "chunk": {
      if (!prev.some((e) => e.id === classified.id && e.kind === "invention"))
        return prev;
      return prev.map((e) => {
        if (e.id !== classified.id || e.kind !== "invention") return e;
        const next = classified.data as FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk;
        const merged = e.chunk
          ? safeMerge(e.chunk, next, functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged)
          : next;
        return { ...e, chunk: merged };
      });
    }
  }
}

function handleLaboratoryExecution(
  prev: ViewerEntry[],
  classified: Classified,
  ts: number,
): ViewerEntry[] {
  switch (classified.type) {
    case "begin":
      return [
        ...prev,
        {
          kind: "laboratory",
          id: classified.id,
          request: classified.data,
          chunk: null,
          error: null,
          timestamp: ts,
        },
      ];
    case "error": {
      if (!prev.some((e) => e.id === classified.id)) return prev;
      return prev.map((e) =>
        e.id === classified.id ? { ...e, error: classified.data } : e,
      );
    }
    case "chunk": {
      if (!prev.some((e) => e.id === classified.id && e.kind === "laboratory"))
        return prev;
      return prev.map((e) => {
        if (e.id !== classified.id || e.kind !== "laboratory") return e;
        const next = classified.data as LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk;
        const merged = e.chunk
          ? safeMerge(e.chunk, next, laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged)
          : next;
        return { ...e, chunk: merged };
      });
    }
  }
}
