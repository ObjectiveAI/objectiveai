/**
 * The daemon connection coordinates the viewer's per-component
 * listeners are built from: the daemon's WebSocket base address and
 * the pre-derived auth signature, handed over by the Rust side via
 * the `websocket_config` Tauri command (the Rust process holds no
 * daemon stream itself — the webview connects directly).
 *
 * There is deliberately NO global listener singleton: App fetches
 * this once and threads it (and/or data derived from it) down as
 * props; components construct and own their own listeners.
 */
import { tauriInvoke } from "./tauri";

export interface DaemonConnection {
  /** The daemon's published base address, e.g. `ws://127.0.0.1:49152`. */
  address: string;
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, or `null`
   * against a secretless daemon. */
  signature: string | null;
}

/** Fetch the daemon connection coordinates. `null` outside Tauri
 * (browser dev) — no daemon is reachable there. */
export async function daemonConnection(): Promise<DaemonConnection | null> {
  const config = await tauriInvoke<{
    address: string;
    signature: string | null;
  }>("websocket_config");
  if (!config) return null;
  return { address: config.address, signature: config.signature };
}
