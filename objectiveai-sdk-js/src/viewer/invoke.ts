/**
 * The viewer-iframe → host cli invocation transport. Lives in its own
 * module (rather than `./index`) so the generated execute functions
 * under `./command/` can import it without a circular dependency on
 * the viewer barrel.
 */

interface CliCommandMessage {
  kind: "plugin-event";
  type: "cli_command";
  value: unknown;
}

/**
 * Shared machinery behind [`invokeCliRequest`]: an AsyncIterable over
 * `cli_command` event values from the host, terminating on the host's
 * synthetic `{"type":"end"}` marker. The `post` callback fires the
 * actual request to the host exactly once, after the message listener
 * is attached (so no line can be missed).
 */
function cliCommandIterable(post: () => void): AsyncIterable<unknown> {
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
        queue.push(msg.value);
        // End-of-stream signal: the HOST synthesizes {"type":"end"}
        // when the spawned cli's stdout closes — the cli itself never
        // emits it.
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
        if (typeof window !== "undefined") {
          window.removeEventListener("message", onMessage);
        }
      };

      if (typeof window === "undefined" || window.parent === window) {
        // Standalone dev mode: no host to invoke the cli. Warn and
        // produce an empty iterator.
        // eslint-disable-next-line no-console
        console.warn(
          "@objectiveai/sdk/viewer: cli invocation outside an iframe; " +
            "no host present, the iterator will yield nothing.",
        );
        done = true;
      } else {
        window.addEventListener("message", onMessage);
        post();
      }

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
 * Run a typed cli request on the host. `request` is the serde JSON of
 * the Rust SDK's `cli::command::Request` enum (the `path`-tagged
 * shape); the host deserializes it and lowers it to argv via
 * `into_command()` — Request→argv stays canonical in Rust, and raw
 * argv invocation deliberately does not exist on this side.
 *
 * Returns an `AsyncIterable<unknown>` over the JSONL output lines the
 * cli emits: `{"type":"error",…}` envelopes or response lines,
 * terminated by the host's synthetic `{"type":"end"}` marker.
 *
 * The plugin author never specifies `destination` — the host bridge
 * derives it from the iframe's identity. To run multiple cli
 * invocations sequentially, `await` each iterator to completion
 * before starting the next; concurrent invocations from the same
 * iframe produce interleaved streams that this shim cannot demux
 * (no per-invocation sub_type on `cli_command` events).
 *
 * Requires the objectiveai cli to be installed on the host
 * (`~/.objectiveai/objectiveai`). When the spawn fails the host
 * streams back a `{"type":"error",…}` envelope followed by
 * `{"type":"end"}`, so the iterator still terminates.
 *
 * This is the transport behind the generated viewer execute
 * functions; plugin authors normally call those instead.
 */
export function invokeCliRequest(request: unknown): AsyncIterable<unknown> {
  return cliCommandIterable(() => {
    // The bridge derives `destination` from event.source — caller
    // never sets it.
    window.parent.postMessage({ kind: "cli-execute", request }, "*");
  });
}
