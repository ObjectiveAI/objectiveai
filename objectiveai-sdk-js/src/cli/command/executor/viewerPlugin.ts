import type { CliCommandRequest } from "../request";

interface CliCommandMessage {
  kind: "plugin-event";
  type: "cli_command";
  /** The invocation id this line belongs to — minted by the executor
   * when it posted the request; the host echoes it on every line. */
  id: string;
  value: unknown;
}

let nextInvocation = 0;

/** A per-invocation id, unique within this iframe's lifetime (a
 * counter plus a random suffix so ids also never collide across an
 * iframe reload mid-run). */
function invocationId(): string {
  nextInvocation += 1;
  return `invocation-${nextInvocation}-${Math.random().toString(36).slice(2)}`;
}

/**
 * Shared machinery behind {@link ViewerPluginExecutor.execute}: an
 * AsyncIterable over `cli_command` event values from the host,
 * terminating on the host's synthetic `{"type":"end"}` marker. The
 * `message` listener is attached before the request is posted, so no
 * line can be missed. Only lines carrying THIS invocation's `id` are
 * consumed — concurrent invocations demux cleanly.
 */
function cliCommandIterable(
  id: string,
  request: CliCommandRequest,
): AsyncIterable<unknown> {
  return {
    [Symbol.asyncIterator]() {
      const queue: unknown[] = [];
      let resolveNext: (() => void) | null = null;
      let done = false;
      let cleaned = false;

      const onMessage = (event: MessageEvent) => {
        const msg = event.data as CliCommandMessage | null;
        if (!msg || typeof msg !== "object") return;
        if (msg.kind !== "plugin-event" || msg.type !== "cli_command") return;
        // Demux: this iterator only consumes its own invocation's lines.
        if (msg.id !== id) return;
        queue.push(msg.value);
        // End-of-stream signal: the HOST synthesizes {"type":"end"}
        // when the run's stream closes — the cli itself never emits it.
        if (
          typeof msg.value === "object" &&
          msg.value !== null &&
          (msg.value as { type?: string }).type === "end"
        ) {
          done = true;
        }
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };

      const cleanup = () => {
        if (cleaned) return;
        cleaned = true;
        window.removeEventListener("message", onMessage);
      };

      window.addEventListener("message", onMessage);
      // The bridge derives the plugin's identity from event.source —
      // the caller never names itself. The id is this invocation's
      // demux key.
      window.parent.postMessage({ kind: "cli-execute", id, request }, "*");

      return {
        async next(): Promise<IteratorResult<unknown>> {
          while (queue.length === 0 && !done) {
            await new Promise<void>((r) => {
              resolveNext = r;
            });
          }
          if (queue.length > 0) {
            return { value: queue.shift(), done: false };
          }
          cleanup();
          return { value: undefined, done: true };
        },
        async return(): Promise<IteratorResult<unknown>> {
          cleanup();
          return { value: undefined, done: true };
        },
      };
    },
  };
}

/**
 * PLUGIN-ONLY executor: runs a typed cli request on the host viewer
 * from inside a plugin's iframe and streams the host's `cli_command`
 * response lines back. Constructing it anywhere else throws — in the
 * main viewer, use `WebSocketExecutor` (the daemon is directly
 * reachable there).
 *
 * `request` is the serde JSON of the Rust SDK's
 * `cli::command::Request`; the iframe posts it to the host bridge as
 * `{kind: "cli-execute", id, request}`, the HOST runs it (the plugin
 * never sees daemon credentials), and every JSONL output line
 * (`{"type":"error",…}` envelopes or response lines) comes back as a
 * `cli_command` plugin-event, terminated by the host's synthetic
 * `{"type":"end"}` marker.
 *
 * The plugin author never specifies a destination — the host bridge
 * derives it from the iframe's identity. Concurrent invocations are
 * fully supported: every invocation mints its own id, the host echoes
 * it on each response line, and each iterator consumes only its own
 * lines.
 *
 * When the request fails host-side, the host streams back a
 * `{"type":"error",…}` envelope followed by `{"type":"end"}`, so the
 * stream still terminates.
 */
export class ViewerPluginExecutor {
  constructor() {
    if (typeof window === "undefined" || window.parent === window) {
      throw new Error(
        "ViewerPluginExecutor is plugin-only: it posts requests to the host " +
          "viewer above this iframe, and there is no host here. (In the " +
          "main viewer, use WebSocketExecutor.)",
      );
    }
  }

  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    return cliCommandIterable(invocationId(), request);
  }
}
