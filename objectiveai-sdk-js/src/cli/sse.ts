/**
 * Shared SSE transport for the daemon's HTTP routes: the command
 * channel (`POST /execute`), the broadcast (`GET /listen`), and the
 * materialized watcher listeners (`/laboratories/list`,
 * `/laboratories/{id}`, `/agents/instances/list`,
 * `/agents/instances/{aih}`). All serve Server-Sent Events; auth rides
 * the `X-OBJECTIVEAI-SIGNATURE` header. The daemon publishes an
 * `http://` address, so the URL is used verbatim (its one remaining
 * WebSocket, `/laboratory`, is dialed by the laboratory host, not here).
 *
 * `connectSse` resolves once the response headers arrive (rejecting on
 * a connection failure or non-OK status — e.g. a refused signature) and
 * yields each SSE event's raw `data` payload so each caller keeps its
 * own parse + fold. Pass `{ method: "POST", body }` for `/execute`.
 */

/**
 * Open the daemon SSE stream at `url` and return an async generator of
 * each event's raw `data` string. `signature`, when present, is sent as
 * the `X-OBJECTIVEAI-SIGNATURE` header. `signal` aborts the stream
 * (call `AbortController.abort()` to close). `init` sets the HTTP method
 * and body (a JSON `body` also sets `Content-Type: application/json`) —
 * omit it for the plain `GET` watcher/broadcast routes. Rejects on a
 * connection failure or a non-2xx status.
 */
export async function connectSse(
  url: string,
  signature: string | null | undefined,
  signal: AbortSignal,
  init?: { method?: string; body?: string },
): Promise<AsyncGenerator<string>> {
  const headers: Record<string, string> = { Accept: "text/event-stream" };
  if (signature) headers["X-OBJECTIVEAI-SIGNATURE"] = signature;
  if (init?.body !== undefined) headers["Content-Type"] = "application/json";
  const response = await fetch(url, {
    method: init?.method ?? "GET",
    ...(init?.body !== undefined ? { body: init.body } : {}),
    headers,
    signal,
  });
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
