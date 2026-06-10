"use client";

import { useState, useRef, useCallback, useEffect } from "react";

export type StreamState = "idle" | "streaming" | "done" | "error";

export interface SDKStreamConfig<TChunk> {
  createStream: (signal: AbortSignal) => Promise<AsyncIterable<TChunk>>;
  merge: (acc: TChunk, next: TChunk) => TChunk;
  deps: unknown[];
  onStreamStart?: () => void;
}

export interface SDKStreamResult<TChunk> {
  chunk: TChunk | null;
  state: StreamState;
  error: string | null;
  start: () => void;
  abort: () => void;
}

export function useSDKStream<TChunk>({
  createStream,
  merge,
  deps,
  onStreamStart,
}: SDKStreamConfig<TChunk>): SDKStreamResult<TChunk> {
  const [chunk, setChunk] = useState<TChunk | null>(null);
  const [state, setState] = useState<StreamState>("idle");
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  const start = useCallback(async () => {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;

    setState("streaming");
    setError(null);
    setChunk(null);
    onStreamStart?.();

    try {
      const stream = await createStream(controller.signal);
      let acc: TChunk | null = null;
      for await (const c of stream) {
        if (controller.signal.aborted) break;
        acc = acc === null ? c : merge(acc, c);
        setChunk(acc);
      }
      if (!controller.signal.aborted) {
        setState("done");
      }
    } catch (err: unknown) {
      if ((err as Error)?.name === "AbortError") return;
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setState("error");
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  const abort = useCallback(() => {
    abortRef.current?.abort();
    setState("idle");
  }, []);

  useEffect(() => {
    return () => { abortRef.current?.abort(); };
  }, []);

  return { chunk, state, error, start, abort };
}
