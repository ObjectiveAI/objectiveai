import type { CliCommandRequest } from "../request";

interface CliCommandMessage {
  kind: "plugin-event";
  type: "cli_command";
  /** The invocation id this line belongs to — minted by the executor
   * when it posted the request; the host echoes it on every line. */
  id: string;
  value: unknown;
}

/** The `"objectiveai"` Tauri channel's event payload shape (the serde
 * JSON of the Rust host's `viewer::Event`), as far as the executor
 * cares: `cli_command` events carry one response line in `value` and
 * the caller-minted invocation id. */
interface ObjectiveaiChannelPayload {
  type?: unknown;
  id?: unknown;
  value?: unknown;
}

let nextInvocation = 0;

/** A per-invocation id, unique within this window's lifetime (a
 * counter plus a random suffix so ids also never collide across a
 * reload mid-run). */
function invocationId(): string {
  nextInvocation += 1;
  return `invocation-${nextInvocation}-${Math.random().toString(36).slice(2)}`;
}

/** The host's synthetic `{"type":"end"}` stream terminator. */
function isEndMarker(value: unknown): boolean {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { type?: string }).type === "end"
  );
}

/**
 * Shared machinery behind {@link ViewerCommandExecutor.execute}: an
 * AsyncIterable over `cli_command` response lines, terminating on the
 * host's synthetic `{"type":"end"}` marker. Only lines carrying THIS
 * invocation's `id` are consumed — concurrent invocations demux
 * cleanly. Two transports, picked by context:
 *
 * - IFRAME (plugin): a `message` listener for the host bridge's
 *   `plugin-event` posts, then a `cli-execute` postMessage to the
 *   parent — the request fires only after the listener is attached,
 *   so no line can be missed.
 * - MAIN VIEWER (top-level window): a Tauri listener on the host's
 *   `"objectiveai"` channel, then a direct `cli_execute` invoke with
 *   `destination: "objectiveai"` — the invoke fires only after the
 *   channel subscription resolves. An invoke rejection (the run never
 *   started) surfaces as one in-band error line, then the end marker.
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
      let cleanup: () => void = () => {};

      const push = (value: unknown) => {
        if (done) return;
        queue.push(value);
        // End-of-stream signal: the HOST synthesizes {"type":"end"}
        // when the run's stream closes — the cli itself never emits it.
        if (isEndMarker(value)) {
          done = true;
        }
        if (resolveNext) {
          const r = resolveNext;
          resolveNext = null;
          r();
        }
      };

      /** Transport failure before/around the run: one in-band error
       * line, then the end marker, so iteration always terminates. */
      const fail = (message: string) => {
        push({ type: "error", level: "error", fatal: null, message });
        push({ type: "end" });
      };

      if (typeof window === "undefined") {
        // No window at all: nothing to talk to.
        // eslint-disable-next-line no-console
        console.warn(
          "@objectiveai/sdk ViewerCommandExecutor: cli invocation without a " +
            "window; the iterator will yield nothing.",
        );
        done = true;
      } else if (window.parent !== window) {
        // IFRAME transport: listener first, then post — synchronous,
        // so no line can be missed.
        const onMessage = (event: MessageEvent) => {
          const msg = event.data as CliCommandMessage | null;
          if (!msg || typeof msg !== "object") return;
          if (msg.kind !== "plugin-event" || msg.type !== "cli_command")
            return;
          // Demux: this iterator only consumes its own invocation's lines.
          if (msg.id !== id) return;
          push(msg.value);
        };
        window.addEventListener("message", onMessage);
        cleanup = () => window.removeEventListener("message", onMessage);
        // The bridge derives `destination` from event.source — the
        // caller never sets it. The id is this invocation's demux key.
        window.parent.postMessage({ kind: "cli-execute", id, request }, "*");
      } else {
        // MAIN VIEWER transport: subscribe to the "objectiveai"
        // channel, then invoke — awaited in order, so no line can be
        // missed.
        let unlisten: (() => void) | null = null;
        cleanup = () => {
          unlisten?.();
          unlisten = null;
        };
        void (async () => {
          try {
            const [{ listen }, { invoke }] = await Promise.all([
              import("@tauri-apps/api/event"),
              import("@tauri-apps/api/core"),
            ]);
            const stop = await listen<ObjectiveaiChannelPayload>(
              "objectiveai",
              (event) => {
                const payload = event.payload;
                if (!payload || typeof payload !== "object") return;
                if (payload.type !== "cli_command") return;
                // Demux: only this invocation's lines.
                if (payload.id !== id) return;
                push(payload.value);
              },
            );
            if (cleaned) {
              stop();
              return;
            }
            unlisten = stop;
            await invoke("cli_execute", {
              request,
              id,
              destination: "objectiveai",
            });
          } catch (e) {
            fail(String(e));
          }
        })();
      }

      const finish = () => {
        if (cleaned) return;
        cleaned = true;
        cleanup();
      };

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
          finish();
          return { value: undefined, done: true };
        },
        async return(): Promise<IteratorResult<unknown>> {
          finish();
          return { value: undefined, done: true };
        },
      };
    },
  };
}

/**
 * Viewer executor: runs a typed cli request on the viewer host and
 * streams the host's `cli_command` response lines back. Takes no
 * construction options and works in BOTH viewer contexts:
 *
 * - inside a plugin iframe, it posts `cli-execute` to the host bridge,
 *   which stamps the plugin's identity as the destination;
 * - inside the main viewer window, it invokes the host's `cli_execute`
 *   Tauri command directly with `destination: "objectiveai"`.
 *
 * `request` is the serde JSON of the Rust SDK's
 * `cli::command::Request`; the host runs it through the daemon's
 * `/execute` route and forwards each JSONL output line
 * (`{"type":"error",…}` envelopes or response lines) as a
 * `cli_command` event, terminated by the host's synthetic
 * `{"type":"end"}` marker.
 *
 * Concurrent invocations are fully supported: every invocation mints
 * its own id, the host echoes it on each response line, and each
 * iterator consumes only its own lines.
 *
 * When the request fails host-side, the host streams back a
 * `{"type":"error",…}` envelope followed by `{"type":"end"}`, so the
 * stream still terminates. Without a window (SSR) it warns and yields
 * nothing.
 */
export class ViewerCommandExecutor {
  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    return cliCommandIterable(invocationId(), request);
  }
}
