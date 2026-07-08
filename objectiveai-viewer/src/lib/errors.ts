/**
 * The viewer's error inbox — anything that would otherwise die in a
 * silent `catch` (connection failures, render throws, unhandled
 * rejections) reports here, and the bottom-right
 * [`ErrorToast`](../components/ErrorToast.tsx) surfaces it: a badge
 * you can click for the details. Pure UI plumbing (a tiny module
 * store), deliberately NOT part of the daemon-data flow.
 *
 * Flood control: identical consecutive (source, message) reports
 * coalesce into one entry with a bumped count (a 1s reconnect loop
 * against a dead daemon would otherwise pile up one entry per
 * second), and only the most recent [`CAP`] entries are kept.
 */

export interface ReportedError {
  id: number;
  /** RFC-ish local time of the FIRST occurrence. */
  at: string;
  /** Where it came from — e.g. `agents list`, `agent <aih>`, `window`. */
  source: string;
  message: string;
  /** Stack / stringified payload, when there is one. */
  detail: string | null;
  /** How many consecutive identical reports coalesced into this entry. */
  count: number;
}

const CAP = 50;

let nextId = 1;
let errors: ReportedError[] = [];
const subscribers = new Set<() => void>();

function notify(): void {
  for (const subscriber of [...subscribers]) {
    subscriber();
  }
}

/** Report one error. `error` may be anything thrown/rejected. */
export function reportError(source: string, error: unknown): void {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === "string"
        ? error
        : JSON.stringify(error) ?? String(error);
  const detail =
    error instanceof Error && typeof error.stack === "string"
      ? error.stack
      : null;
  const last = errors[errors.length - 1];
  if (last !== undefined && last.source === source && last.message === message) {
    // Coalesce the repeat instead of flooding.
    errors = [...errors.slice(0, -1), { ...last, count: last.count + 1 }];
  } else {
    errors = [
      ...errors.slice(Math.max(0, errors.length - (CAP - 1))),
      {
        id: nextId++,
        at: new Date().toLocaleTimeString(),
        source,
        message,
        detail,
        count: 1,
      },
    ];
  }
  notify();
}

/** The current error list, oldest first. Identity-stable between
 * changes (safe for `useSyncExternalStore`). */
export function reportedErrors(): readonly ReportedError[] {
  return errors;
}

/** Clear the inbox. */
export function clearErrors(): void {
  errors = [];
  notify();
}

/** Subscribe to inbox changes; returns the unsubscribe. */
export function subscribeErrors(callback: () => void): () => void {
  subscribers.add(callback);
  return () => {
    subscribers.delete(callback);
  };
}

let globalHandlersInstalled = false;

/** Route uncaught window errors and unhandled promise rejections into
 * the inbox (idempotent — StrictMode safe). This is what catches the
 * deliberate `unhandled … kind` throws in the conversation renderers. */
export function installGlobalErrorHandlers(): void {
  if (globalHandlersInstalled) return;
  globalHandlersInstalled = true;
  window.addEventListener("error", (event) => {
    reportError("window", event.error ?? event.message);
  });
  window.addEventListener("unhandledrejection", (event) => {
    reportError("unhandled rejection", event.reason);
  });
}
