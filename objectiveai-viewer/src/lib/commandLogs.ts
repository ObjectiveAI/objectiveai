// The TS mirror of the Rust command-logs sink
// (src-tauri/src/shell/command_logs.rs). Rust holds the daemon's
// /listen stream for the viewer's whole life and writes it two-level:
// a root requests file (one line per command run, WITH the producer's
// identity) plus one items file per request. Same flow contract as
// viewer-logs: pulls stream the files BACKWARDS (newest first — they
// PREPEND history) while the live `command-logs://request` /
// `command-logs://item` events APPEND the present; everything keys by
// seq/id and the JS side owns memory bounds.

import { isTauri } from "./tauri";

/** One captured request announcement — the command-logs list row. */
export interface CommandRequestEntry {
  /** Monotonic within one viewer run — the ordering key. */
  seq: number;
  /** Epoch millis, stamped by Rust on receipt. */
  at_ms: number;
  /** The broadcast stream id — names the items file; the upsert key. */
  id: string;
  /** The command path (the request's `path_type`), when present. */
  path?: string;
  agent_instance_hierarchy?: string;
  agent_id?: string;
  agent_full_id?: string;
  agent_remote?: string;
  response_id?: string;
  response_ids?: string;
  plugin_owner?: string;
  plugin_name?: string;
  plugin_version?: string;
  /** Fired by the task scheduler. */
  task: boolean;
  /** The run's actual request, verbatim. */
  request: unknown;
}

/** One line of a request's stream: a response item, or the end. */
export interface CommandItemEntry {
  /** Monotonic within the request's stream — the upsert key. */
  seq: number;
  /** Epoch millis, stamped by Rust on receipt. */
  at_ms: number;
  /** The response item, verbatim (absent on the end terminator). */
  value?: unknown;
  /** This line is the run's REQUEST — exactly one, first (absent
   * only when the announcement predated the viewer run). */
  request?: boolean;
  /** The stream ended — exactly one, last. */
  end?: boolean;
}

/** The `command-logs://item` payload: the id routes each item to the
 * tab watching that request. */
export interface CommandItemEvent {
  request_id: string;
  item: CommandItemEntry;
}

/** Stream up to `count` of this run's captured requests, NEWEST
 * FIRST, into `onEntry`. Resolves when the stream is done. */
export async function commandLogsPull(
  count: number,
  onEntry: (entry: CommandRequestEntry) => void,
): Promise<void> {
  if (!isTauri()) return;
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const channel = new Channel<CommandRequestEntry>();
  channel.onmessage = onEntry;
  await invoke("command_logs_pull", { count, onEntry: channel });
}

/** Stream up to `count` of ONE request's items, NEWEST FIRST, into
 * `onItem`. Resolves when the stream is done. */
export async function commandLogItemsPull(
  id: string,
  count: number,
  onItem: (item: CommandItemEntry) => void,
): Promise<void> {
  if (!isTauri()) return;
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const channel = new Channel<CommandItemEntry>();
  channel.onmessage = onItem;
  await invoke("command_log_items_pull", { id, count, onItem: channel });
}
