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

/** A non-2xx response to an SSE connect — carries the HTTP status so
 * callers can map it (e.g. the per-channel stream's 404/409/401). */
export class SseHttpError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
    this.name = "SseHttpError";
  }
}

/**
 * Open the daemon SSE stream at `url` and return an async generator of
 * each event's raw `data` string. `signature`, when present, is sent as
 * the `X-OBJECTIVEAI-SIGNATURE` header. `signal` aborts the stream
 * (call `AbortController.abort()` to close). `init` sets the HTTP method,
 * body (a JSON `body` also sets `Content-Type: application/json`), and
 * the per-run agent-identity header values — one `X-OBJECTIVEAI-*`
 * header per set field, the same names the api stamps on outbound
 * calls. Omit `init` for the plain `GET` watcher/broadcast routes.
 * Rejects on a connection failure, or with {@link SseHttpError} on a
 * non-2xx status.
 */
export async function connectSse(
  url: string,
  signature: string | null | undefined,
  signal: AbortSignal,
  init?: {
    method?: string;
    body?: string;
    /** Value for the `X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY` header. */
    agentInstanceHierarchy?: string | null;
    /** Value for the `X-OBJECTIVEAI-AGENT-ID` header. */
    agentId?: string | null;
    /** Value for the `X-OBJECTIVEAI-AGENT-FULL-ID` header. */
    agentFullId?: string | null;
    /** Value for the `X-OBJECTIVEAI-AGENT-REMOTE` header (the
     * JSON-encoded `RemotePath`). */
    agentRemote?: string | null;
    /** Value for the `X-OBJECTIVEAI-RESPONSE-ID` header. */
    responseId?: string | null;
    /** Value for the `X-OBJECTIVEAI-RESPONSE-IDS` header. */
    responseIds?: string | null;
    /** Value for the `X-OBJECTIVEAI-CHANNEL-SECRET` header — a channel
     * secret (`S_pub` or `S_owner`) authorizing a per-channel observer
     * stream. */
    channelSecret?: string | null;
  },
): Promise<AsyncGenerator<string>> {
  const headers: Record<string, string> = { Accept: "text/event-stream" };
  if (signature) headers["X-OBJECTIVEAI-SIGNATURE"] = signature;
  if (init?.body !== undefined) headers["Content-Type"] = "application/json";
  if (init?.agentInstanceHierarchy) {
    headers["X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY"] =
      init.agentInstanceHierarchy;
  }
  if (init?.agentId) headers["X-OBJECTIVEAI-AGENT-ID"] = init.agentId;
  if (init?.agentFullId) {
    headers["X-OBJECTIVEAI-AGENT-FULL-ID"] = init.agentFullId;
  }
  if (init?.agentRemote) {
    headers["X-OBJECTIVEAI-AGENT-REMOTE"] = init.agentRemote;
  }
  if (init?.responseId) {
    headers["X-OBJECTIVEAI-RESPONSE-ID"] = init.responseId;
  }
  if (init?.responseIds) {
    headers["X-OBJECTIVEAI-RESPONSE-IDS"] = init.responseIds;
  }
  if (init?.channelSecret) {
    headers["X-OBJECTIVEAI-CHANNEL-SECRET"] = init.channelSecret;
  }
  let response: Response;
  try {
    response = await fetch(url, {
      method: init?.method ?? "GET",
      ...(init?.body !== undefined ? { body: init.body } : {}),
      headers,
      signal,
    });
  } catch (e) {
    throw new Error(
      `connect daemon sse: fetch ${init?.method ?? "GET"} ${url} failed: ${String(e)}`,
    );
  }
  if (!response.ok || !response.body) {
    throw new SseHttpError(
      response.status,
      `connect daemon sse: HTTP ${response.status} from ${init?.method ?? "GET"} ${url}`,
    );
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
