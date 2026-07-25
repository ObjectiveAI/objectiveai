/**
 * The viewer-plugin bundle download —
 * {@link Client.getViewerPlugin}'s result. LOW-LEVEL by design: raw
 * tar.gz bytes, caller-paced, no filesystem — the consumer (the
 * viewer's installer) un-tars into its own staging its own way.
 *
 * Regular mode wraps the fetch response body directly. Viewer mode
 * rides the `daemon_viewer_plugin` proxy command, which streams
 * base64 `chunk` events (the text-SSE proxy shape can't carry binary)
 * — decoded here back into the same byte-stream surface.
 */

/** One in-flight viewer-extension bundle: the tag's commit SHA (from
 * the `X-OBJECTIVEAI-SHA` response header) plus the tar.gz byte
 * stream. A TRUNCATED body is indistinguishable here from a complete
 * one — the caller's un-gzip/un-tar is what validates completeness (a
 * daemon-side failure mid-build never streams: the tar starts only
 * after the build succeeded). */
export class ViewerPlugin {
  /** The plugin tag's commit SHA, when the daemon stamped it. */
  readonly commitSha: string | null;
  readonly #chunks: AsyncGenerator<Uint8Array>;

  /** @internal — minted by {@link Client.getViewerPlugin}. */
  constructor(commitSha: string | null, chunks: AsyncGenerator<Uint8Array>) {
    this.commitSha = commitSha;
    this.#chunks = chunks;
  }

  /** The tar.gz bytes, chunk by chunk. `return()`/`break` aborts the
   * transfer. */
  chunks(): AsyncGenerator<Uint8Array> {
    return this.#chunks;
  }

  [Symbol.asyncIterator](): AsyncGenerator<Uint8Array> {
    return this.#chunks;
  }
}

/** Decode one base64 payload into bytes — the viewer-proxy chunk
 * encoding. `atob` is universal (browser + Node 16+). */
export function decodeBase64Chunk(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
