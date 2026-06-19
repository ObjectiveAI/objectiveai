import type { CliCommandRequest } from "../request";

// In-process plugin executor: a plugin authored in Node uses this to ask the
// host to run CLI commands over the process's own stdin/stdout, speaking the
// NDJSON command protocol. It is a direct port of Rust `PluginCommandExecutor`, and
// reproduces the four thread-safety primitives that make PARALLEL callers
// safe (the whole point — many concurrent `execute()` calls must not corrupt
// the single shared stream):
//
//   1. ONE reader. A single `readline` listener owns `process.stdin`; no
//      caller ever reads stdin directly, so concurrent calls never race on
//      the read. (Rust: the single `spawn_listener` task.)
//   2. id -> stream demux. Each call mints a monotonic id and registers a
//      channel in `pending`; the listener routes each line to the matching
//      id's channel. (Rust: `DashMap<id, mpsc::Sender>`.)
//   3. Serialized writes. All writes to stdout go through one promise chain
//      (`writeChain`) so two concurrent callers can't interleave partial
//      NDJSON lines. (Rust: `Arc<Mutex<Stdout>>`.)
//   4. Liveness recheck. On stdin EOF the listener sets `alive = false` and
//      ends every pending channel; each `execute()` inserts THEN rechecks
//      `alive`, so a call racing the close can't strand a registered channel.
//      (Rust: `listener_alive` store-before-clear / insert-before-recheck.)
//
// As in Rust, there is exactly one instance per process (it captures the
// global stdin/stdout): use [`PluginCommandExecutor.instance`].

export class PluginCommandExecutor {
  private static _instance: PluginCommandExecutor | undefined;

  private counter = 0;
  private readonly pending = new Map<string, Channel<unknown>>();
  private alive = true;
  private writeChain: Promise<void> = Promise.resolve();
  private listenerReady: Promise<void> | undefined;

  /** The process-wide singleton (captures the global stdin/stdout). */
  static instance(): PluginCommandExecutor {
    return (PluginCommandExecutor._instance ??= new PluginCommandExecutor());
  }

  /**
   * Send `request` to the host and stream back its responses (yielded raw;
   * the caller's `CliStream` discriminates errors vs responses). Safe to call
   * concurrently from any number of parallel callers.
   */
  execute(request: CliCommandRequest): AsyncIterable<unknown> {
    this.ensureListener();

    const id = String(this.counter++);
    const channel = makeChannel<unknown>();
    // (2) register before doing anything else, then (4) recheck liveness.
    this.pending.set(id, channel);
    if (!this.alive) {
      this.pending.delete(id);
      channel.fail(new Error("plugin executor: stdin closed"));
      return channel.iterable;
    }

    // Serialize the request to JSON and pass it as the cli's `--request`
    // argv; the host re-enters its `run` with this command.
    const argv = ["--request", JSON.stringify(request)];

    // (3) serialized write; failures end this call's stream.
    void this.write(JSON.stringify({ type: "command", id, command: argv })).catch(
      (e: unknown) => {
        this.pending.delete(id);
        channel.fail(e);
      },
    );

    return channel.iterable;
  }

  /** Serialize every write to stdout behind the listener being ready. */
  private write(line: string): Promise<void> {
    this.writeChain = this.writeChain
      .then(() => this.listenerReady)
      .then(
        () =>
          new Promise<void>((resolve, reject) => {
            process.stdout.write(`${line}\n`, (err) =>
              err ? reject(err) : resolve(),
            );
          }),
      );
    return this.writeChain;
  }

  /** Start the single stdin reader once. */
  private ensureListener(): void {
    if (this.listenerReady !== undefined) return;
    this.listenerReady = (async (): Promise<void> => {
      const readline = await import("node:readline");
      const rl = readline.createInterface({
        input: process.stdin,
        crlfDelay: Infinity,
      });
      rl.on("line", (line) => this.onLine(line));
      rl.on("close", () => {
        // (4) flag before draining, mirroring the Rust ordering.
        this.alive = false;
        for (const ch of this.pending.values()) ch.end();
        this.pending.clear();
      });
    })();
  }

  /** Route one inbound NDJSON line to the matching id's channel. */
  private onLine(line: string): void {
    if (line.trim().length === 0) return;
    let parsed: unknown;
    try {
      parsed = JSON.parse(line);
    } catch {
      return;
    }
    if (typeof parsed !== "object" || parsed === null) return;
    const obj = parsed as Record<string, unknown>;
    const id = obj.id;
    if (typeof id !== "string") return;
    const channel = this.pending.get(id);
    if (channel === undefined) return;

    // Terminal markers: SDK form `{id, done:true}` or host form
    // `{id, value:{type:"command_complete", exit_code}}`.
    if (obj.done === true || isCommandComplete(obj.value)) {
      this.pending.delete(id);
      channel.end();
      return;
    }
    // Yield each value item RAW; error envelopes pass through as values and
    // the caller's `CliStream` discriminates them via the `CliError` union.
    if ("value" in obj) channel.push(obj.value);
  }
}

function isCommandComplete(value: unknown): boolean {
  return (
    typeof value === "object" &&
    value !== null &&
    (value as { type?: unknown }).type === "command_complete"
  );
}

// A single-producer / single-consumer async queue. Each concurrent
// `execute()` call gets its own channel; the shared stdin listener pushes
// that call's demultiplexed items into it (the JS analogue of the per-id
// `mpsc::unbounded_channel` in Rust `PluginCommandExecutor`).
interface Channel<T> {
  push(value: T): void;
  end(): void;
  fail(error: unknown): void;
  iterable: AsyncIterable<T>;
}

function makeChannel<T>(): Channel<T> {
  const queue: T[] = [];
  let done = false;
  let failed = false;
  let error: unknown;
  let wake: (() => void) | undefined;

  const signal = (): void => {
    const w = wake;
    wake = undefined;
    w?.();
  };

  return {
    push(value: T): void {
      if (done) return;
      queue.push(value);
      signal();
    },
    end(): void {
      done = true;
      signal();
    },
    fail(e: unknown): void {
      failed = true;
      error = e;
      done = true;
      signal();
    },
    iterable: {
      async *[Symbol.asyncIterator](): AsyncIterator<T> {
        for (;;) {
          while (queue.length > 0) {
            yield queue.shift() as T;
          }
          if (failed) throw error;
          if (done) return;
          await new Promise<void>((resolve) => {
            wake = resolve;
          });
        }
      },
    },
  };
}
