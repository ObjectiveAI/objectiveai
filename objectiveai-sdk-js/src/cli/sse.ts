/**
 * Shared SSE transport for the daemon's materialized watcher listeners
 * (`/laboratories/list`, `/laboratories/{id}`, `/agents/instances/list`,
 * `/agents/instances/{aih}`). These routes serve Server-Sent Events;
 * auth rides the `X-OBJECTIVEAI-SIGNATURE` header (browser `fetch` can
 * set headers, so the old first-frame WebSocket preamble is gone).
 *
 * `connectSse` resolves once the response headers arrive (rejecting on
 * a connection failure or non-OK status — e.g. a refused signature),
 * mirroring the old WebSocket handshake, and yields each SSE event's
 * raw `data` payload so each listener keeps its own parse + fold.
 */

/** Rewrite a `ws://`/`wss://` URL to `http`/`https` (`fetch` cannot
 * dial a `ws://` URL); other schemes pass through unchanged. */
export function wsToHttp(url: string): string {
  if (url.startsWith("ws://")) return "http://" + url.slice("ws://".length);
  if (url.startsWith("wss://")) return "https://" + url.slice("wss://".length);
  return url;
}

/**
 * Open the daemon SSE stream at `url` and return an async generator of
 * each event's raw `data` string. `signature`, when present, is sent as
 * the `X-OBJECTIVEAI-SIGNATURE` header. `signal` aborts the stream
 * (call `AbortController.abort()` to close). Rejects on a connection
 * failure or a non-2xx status.
 */
export async function connectSse(
  url: string,
  signature: string | null | undefined,
  signal: AbortSignal,
): Promise<AsyncGenerator<string>> {
  const headers: Record<string, string> = { Accept: "text/event-stream" };
  if (signature) headers["X-OBJECTIVEAI-SIGNATURE"] = signature;
  const response = await fetch(wsToHttp(url), { headers, signal });
  if (!response.ok || !response.body) {
    throw new Error(`connect daemon sse: HTTP ${response.status}`);
  }
  return readSseData(response.body);
}

/** Read an SSE body and yield each event's raw `data:` payload (the
 * text after `data:`), one string per data line. Splits on the SSE
 * event separator (`\n\n`); ignores comment (`:`) and other fields. */
async function* readSseData(
  body: ReadableStream<Uint8Array>,
): AsyncGenerator<string> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) {
        // Flush any trailing complete event.
        for (const data of dataLines(buffer)) yield data;
        break;
      }
      buffer += decoder.decode(value, { stream: true });
      const parts = buffer.split(/\n\n/);
      // Keep the last (possibly incomplete) part buffered.
      buffer = parts.pop() ?? "";
      for (const part of parts) {
        for (const data of dataLines(part)) yield data;
      }
    }
  } finally {
    reader.releaseLock();
  }
}

/** Extract the `data:` payloads from one SSE event block. */
function dataLines(block: string): string[] {
  const out: string[] = [];
  for (const line of block.split("\n")) {
    if (line.startsWith("data:")) out.push(line.slice("data:".length).trim());
  }
  return out;
}
