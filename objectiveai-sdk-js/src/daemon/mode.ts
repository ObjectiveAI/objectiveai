/**
 * The Client's two transports behind one seam. Every daemon surface
 * branches ONCE on this union:
 *
 * - `regular` — fetch+SSE straight at the daemon's published
 *   `http://` address; this side stamps the auth signature and (where
 *   a surface carries one) the caller identity headers.
 * - `viewer` — everything rides the injected Tauri transport; the
 *   viewer's Rust proxy owns address, signature, AND identity — this
 *   mode stamps NOTHING (the standing viewer rule: secrets and
 *   identity never enter the webview).
 */

import type { Identity } from "../identity";
import { connectSse } from "./sse";
import { connectViewerStream, type ViewerTransport } from "./viewerStream";

/** The Client's resolved construction — see the module docs. */
export type ClientMode =
  | {
      mode: "regular";
      /** Base address, trailing-slash-trimmed; routes are appended. */
      address: string;
      signature?: string | null;
      identity?: Identity;
    }
  | { mode: "viewer"; transport: ViewerTransport };

/**
 * Open one daemon SSE stream in whichever mode: `route` (+ optional
 * `query`) against the regular address, or the `viewerCommand` proxy
 * with `viewerArgs`. Resolves — like both underlying transports —
 * only once the stream has actually opened (2xx headers / proxy
 * connect success); the generator yields raw `data` payloads.
 */
export async function openStream(
  mode: ClientMode,
  signal: AbortSignal,
  spec: {
    route: string;
    query?: Record<string, string | undefined>;
    viewerCommand: string;
    viewerArgs?: Record<string, unknown>;
  },
): Promise<AsyncGenerator<string>> {
  if (mode.mode === "viewer") {
    return connectViewerStream(
      mode.transport,
      spec.viewerCommand,
      spec.viewerArgs ?? {},
      signal,
    );
  }
  let url = `${mode.address}${spec.route}`;
  const pairs = Object.entries(spec.query ?? {}).filter(
    (pair): pair is [string, string] => pair[1] !== undefined,
  );
  if (pairs.length > 0) {
    url += `?${new URLSearchParams(pairs).toString()}`;
  }
  return connectSse(url, mode.signature, signal);
}
