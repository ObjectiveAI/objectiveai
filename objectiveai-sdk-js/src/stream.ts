import { ObjectiveAIFetchError, isResponseError } from "./error";

/**
 * A readable stream wrapper for Server-Sent Events (SSE).
 * Provides async iteration over parsed SSE data events.
 * Throws ObjectiveAIFetchError if the API returns an error in the stream.
 */
export class Stream<T> implements AsyncIterable<T> {
  private reader: ReadableStreamDefaultReader<Uint8Array>;
  private decoder: TextDecoder;
  private buffer: string = "";
  private done: boolean = false;
  private controller: AbortController | null;

  constructor(response: Response, controller?: AbortController | null) {
    if (!response.body) {
      throw new Error("Response body is null");
    }
    this.reader = response.body.getReader();
    this.decoder = new TextDecoder();
    this.controller = controller ?? null;
  }

  /**
   * Abort the stream.
   */
  abort(): void {
    this.controller?.abort();
  }

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    try {
      while (!this.done) {
        const { value, done } = await this.reader.read();

        if (done) {
          this.done = true;
          // Process any remaining data in buffer
          if (this.buffer.trim()) {
            const events = this.parseSSE(this.buffer);
            for (const event of events) {
              if (event !== null) {
                yield event;
              }
            }
          }
          break;
        }

        this.buffer += this.decoder.decode(value, { stream: true });

        // Split by double newline (SSE event separator)
        const parts = this.buffer.split(/\n\n/);

        // Keep the last part in the buffer (might be incomplete)
        this.buffer = parts.pop() ?? "";

        // Process complete events
        for (const part of parts) {
          const events = this.parseSSE(part);
          for (const event of events) {
            if (event !== null) {
              yield event;
            }
          }
        }
      }
    } finally {
      this.reader.releaseLock();
    }
  }

  /**
   * Parse SSE format and extract data.
   * Returns null for [DONE] or empty events.
   *
   * SSE format:
   * - Lines starting with `:` are comments and are ignored
   * - `data:` lines contain the event payload
   * - Empty lines separate events
   * - We only process `data:` fields, ignoring `event:`, `id:`, `retry:`
   */
  private parseSSE(text: string): (T | null)[] {
    const results: (T | null)[] = [];
    const lines = text.split("\n");
    let hasDataLine = false;

    for (const line of lines) {
      // Skip empty lines
      if (!line) {
        continue;
      }

      // Skip comment lines (lines starting with `:`)
      if (line.startsWith(":")) {
        continue;
      }

      // Parse "data: ..." lines
      if (line.startsWith("data:")) {
        hasDataLine = true;
        const data = line.slice(5).trim();

        // Check for stream termination
        if (data === "[DONE]") {
          this.done = true;
          continue;
        }

        // Skip empty data
        if (!data) {
          continue;
        }

        const parsed = JSON.parse(data);

        // Check if this is a ResponseError
        if (isResponseError(parsed)) {
          throw new ObjectiveAIFetchError(parsed);
        }

        results.push(parsed as T);
      }
      // Ignore other SSE fields like event:, id:, retry:
    }

    // If the event block contained only comments (no data lines), return empty
    if (!hasDataLine && results.length === 0) {
      return [];
    }

    return results;
  }

  /**
   * Collect all events into an array.
   */
  async toArray(): Promise<T[]> {
    const results: T[] = [];
    for await (const item of this) {
      results.push(item);
    }
    return results;
  }
}
