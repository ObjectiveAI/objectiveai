/**
 * @objectiveai/sdk/viewer
 *
 * TypeScript shim that lets a plugin's iframe-mounted UI bundle
 * subscribe to events from the host viewer AND invoke the
 * objectiveai CLI in-process on the host.
 *
 * Production context (loaded inside the host viewer):
 *   - The plugin's `index.html` runs inside an `<iframe sandbox>`
 *     pointed at `plugin://localhost/<repo>/`.
 *   - `window.parent` is the host viewer's React app.
 *   - `listen(sub_type, handler)` registers a callback for incoming
 *     `inbound` events forwarded by the host bridge.
 *   - `invokeCli(args)` posts a `cli-invoke` message to the host;
 *     the host runs `objectiveai_cli::run()` and streams each output
 *     line back as a `cli_command` event. The returned AsyncIterable
 *     yields each line and terminates on `{"type":"end"}`.
 *
 * Dev context (plugin author runs their own Tauri shell standalone):
 *   - `window.parent === window` (no host).
 *   - `listen()` falls through to `@tauri-apps/api`'s `listen`.
 *   - `invokeCli()` warns and yields nothing.
 *
 * The plugin author writes the same code in both contexts.
 */
/**
 * Register a handler for incoming `inbound` plugin events. Returns
 * an unsubscribe function.
 *
 * `sub_type` matches the `sub_type` field of the `Event::Inbound`
 * emitted by the Rust host — the string the plugin author
 * registered in their manifest's `viewer_routes` entry, or one of
 * the built-in event names (e.g. `agent_completions`).
 *
 * In iframe context the events come from the host's bridge; in
 * standalone-dev context they come from `@tauri-apps/api`'s `listen`.
 */
declare function listen<T = unknown>(sub_type: string, handler: (value: T) => void): () => void;
/**
 * Invoke `objectiveai-cli` in-process on the host with `args`.
 * Returns an `AsyncIterable<unknown>` over the JSONL output lines
 * the cli emits. Each yielded value is the parsed cli output
 * envelope (e.g. `{"type":"begin"}`, `{"type":"notification","value":...}`,
 * `{"type":"end"}`); the iterator terminates after the `end` line.
 *
 * The plugin author never specifies `destination` — the host bridge
 * derives it from the iframe's identity. To run multiple cli
 * invocations sequentially, `await` each iterator to completion
 * before starting the next; concurrent invocations from the same
 * iframe produce interleaved streams that this shim cannot demux
 * (no per-invocation sub_type on `cli_command` events).
 *
 * Requires the host viewer to be built with the `cli` Cargo feature.
 * Without it, the host's `cli_run` Tauri command isn't registered
 * and the invocation falls through to a Tauri "command not found"
 * error (silently ignored here — the iterator just yields nothing).
 */
declare function invokeCli(args: string[]): AsyncIterable<unknown>;
/** Internal-use: clear in-flight state. Exposed for tests only.
 * Note: the module-level `message` event listener stays attached —
 * removing/re-attaching it would just register a duplicate. The
 * listeners map is what carries per-test state. */
declare function __resetForTests(): void;

export { __resetForTests, invokeCli, listen };
