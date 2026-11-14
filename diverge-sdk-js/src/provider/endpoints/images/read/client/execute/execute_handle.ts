// The read scope, held: the channels a caller opens on it.

import { decodeServerFrame } from "../../../../../../wire/frame/server.js";
import type { Handle, Scope } from "../../../../../../wire/client/index.js";
import { ChannelStream } from "../../../../containers/client/channel_stream.js";
import { OpenError, UnaryError } from "../../../../containers/client/error.js";
import { unary } from "../../../../containers/client/unary.js";
import type { Document } from "../../../../../../shared/containers/info/document.js";
import type { Error as Refusal } from "../../../../../../shared/error.js";
import * as info from "../../../../../../shared/containers/info/response/frame.js";
import * as blob from "../../../../../../shared/containers/oci/blob/response/frame.js";
import * as manifest from "../../../../../../shared/containers/oci/manifest/response/frame.js";
import * as prove from "../../../../../../shared/containers/oci/prove/response/frame.js";
import type * as prove_request from "../../../../../../shared/containers/oci/prove/request/request.js";
import * as channel_request from "../channel_request/index.js";

/** How a held read scope ended. */
export type End = "finished" | "closed";

/** A blob's pieces, each appending, to the channel's finish; none at all is a digest the image does not reach. */
export type BlobStream = ChannelStream<Uint8Array, never>;

/**
 * A read scope answered held: the channels this side opens on it, one
 * per ask, several in flight at once if wanted, and the cancel.
 * Nothing happens on its own: dropping it sends nothing; `cancel` asks
 * for the scope's finish, and `wait` reads it.
 */
export class ExecuteHandle {
  readonly #handle: Handle;
  readonly #scope: Scope;
  #end: Promise<End> | undefined;

  constructor(handle: Handle, scope: Scope) {
    this.#handle = handle;
    this.#scope = scope;
  }

  /** The scope's number. */
  get scope(): number {
    return this.#scope.scope;
  }

  /** The manifest under `digest` the image reaches, with its media type; `undefined` for one it does not. */
  async manifest(digest: string): Promise<manifest.Frame | undefined> {
    try {
      return await unary<manifest.Frame, never>(this.#handle, this.scope, channel_request.encode({ oci_manifest: { digest } }), (payload) => ({
        item: manifest.decode(payload),
      }));
    } catch (error) {
      if (error instanceof UnaryError && error.kind === "unanswered") {
        return undefined;
      }
      throw error;
    }
  }

  /** The blob under `digest` the image reaches, in pieces; a stream with none is one it does not. */
  async blob(digest: string): Promise<BlobStream> {
    let channel;
    try {
      channel = await this.#handle.sendChannelRequest(this.scope, channel_request.encode({ oci_blob: { digest } }));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
    return new ChannelStream<Uint8Array, never>(channel.responseReceiver, (payload) => ({ item: blob.decode(payload) }));
  }

  /** The 32 bytes of the proof over the image, under `request`'s platform and nonce; `undefined` where it is not held. */
  async prove(request: prove_request.Request): Promise<Uint8Array | undefined> {
    try {
      return await unary<Uint8Array, never>(this.#handle, this.scope, channel_request.encode({ oci_prove: request }), (payload) => ({
        item: prove.decode(payload),
      }));
    } catch (error) {
      if (error instanceof UnaryError && error.kind === "unanswered") {
        return undefined;
      }
      throw error;
    }
  }

  /**
   * The image's `/DIVERGE.md`, parsed; `undefined` for an image that
   * ships none. The source reads the file out of the image without
   * running it; a file it cannot read, or that is no document, is the
   * `refused` error carrying the source's own words.
   */
  async info(): Promise<Document | undefined> {
    return await unary<Document | undefined, Refusal>(this.#handle, this.scope, channel_request.encode("info"), (payload) => {
      const answer = info.decode(payload);
      if (answer === "missing") {
        return { item: undefined };
      }
      if ("error" in answer) {
        return { refusal: answer.error };
      }
      return { item: answer.info };
    });
  }

  /** End the scope. Nothing answers on the channel this opens; what answers is the scope's own finish, on `wait`. */
  async cancel(): Promise<void> {
    try {
      await this.#handle.sendChannelRequest(this.scope, channel_request.encode("cancel"));
    } catch (cause) {
      throw new OpenError("send", cause);
    }
  }

  /**
   * The scope's end: `finished` at its finish — the cancel answered, or
   * the source gone — or `closed` where the connection ended first.
   * Resolves once, and answers the same way again after.
   */
  wait(): Promise<End> {
    this.#end ??= this.#read();
    return this.#end;
  }

  async #read(): Promise<End> {
    for (;;) {
      const bytes = await this.#scope.responseReceiver.next();
      if (bytes === undefined) {
        return "closed";
      }
      let envelope;
      try {
        envelope = decodeServerFrame(bytes);
      } catch {
        return "closed";
      }
      if (envelope.kind === "response_finish") {
        return "finished";
      }
    }
  }
}
