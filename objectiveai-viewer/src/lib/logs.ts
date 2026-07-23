// The TS mirror of the Rust log store (src-tauri/src/shell/logs.rs).
// Capture is Rust-side and always on — every webview's console.*,
// uncaught errors, and unhandled rejections land in a capped ring via
// the injected initialization script. The viewer-logs tab is a pure
// view: `logs_snapshot` boot read + `logs://appended` upserts, both
// keyed by `seq` (coalesced repeats re-broadcast the same seq with a
// bumped count).

import { tauriInvoke } from "./tauri";

export interface LogEntry {
  /** Monotonic, never reused — the upsert key. */
  seq: number;
  /** Epoch millis, stamped by Rust on receipt. */
  at_ms: number;
  /** A tab's title, or the chrome webview's label. */
  source: string;
  /** `log`/`info`/`warn`/`error`/`debug`/`trace`, or
   * `uncaught` / `unhandledrejection`. */
  level: string;
  message: string;
  /** Stack trace, when there is one. */
  detail: string | null;
  /** Consecutive identical reports coalesced into this entry. */
  count: number;
}

export function logsSnapshot(): Promise<LogEntry[] | undefined> {
  return tauriInvoke<LogEntry[]>("logs_snapshot");
}
