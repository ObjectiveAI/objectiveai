// The TS mirror of the Rust log sink (src-tauri/src/shell/logs.rs).
// Capture is Rust-side and always on — every webview's console.*,
// uncaught errors, and unhandled rejections are stamped and APPENDED
// to this run's logfile (state/<state>/viewer/viewer-logs/) and
// broadcast as `logs://appended`. History comes from `logs_pull`,
// which STREAMS the file backwards — newest first — through an IPC
// channel, so pulls PREPEND (ever older) while live events APPEND
// (ever newer); both key by `seq`, and the JS side owns the memory
// cap (Rust stays O(1)).

import { isTauri } from "./tauri";

export interface LogEntry {
  /** Monotonic within one viewer run, never reused — the upsert key. */
  seq: number;
  /** Epoch millis, stamped by Rust on receipt. */
  at_ms: number;
  /** A tab's title, or `viewer-container` for the chrome. */
  source: string;
  /** `log`/`info`/`warn`/`error`/`debug`/`trace`, or
   * `uncaught` / `unhandledrejection`. */
  level: string;
  message: string;
  /** Stack trace, when there is one. */
  detail: string | null;
}

/** Stream up to `count` entries of this run's logfile, NEWEST FIRST,
 * into `onEntry`. Resolves when the stream is done. */
export async function logsPull(
  count: number,
  onEntry: (entry: LogEntry) => void,
): Promise<void> {
  if (!isTauri()) return;
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const channel = new Channel<LogEntry>();
  channel.onmessage = onEntry;
  await invoke("logs_pull", { count, onEntry: channel });
}
