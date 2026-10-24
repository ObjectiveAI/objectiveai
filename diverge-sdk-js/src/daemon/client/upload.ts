// Sending files on the channels the daemon opens, and hearing the
// answer.

import { CHUNK_SIZE } from "../../index.js";
import type { Decode, Encode } from "../../wire/index.js";
import { decodeServerFrame } from "../../wire/frame/server.js";
import type { Handle, Scope } from "../../wire/client/index.js";
import * as channel_response from "../../provider/endpoints/volumes/write/client/channel_response/frame.js";
import { OpenError } from "./error.js";
import type { OneShotErrorKind } from "./error.js";
import { open } from "./one_shot.js";

/**
 * A file's content, as a caller supplies it: the whole of it at
 * once, or pieces as they come — a file read stream, a fetch body —
 * each piece sent as it arrives, sliced to `CHUNK_SIZE` where it is
 * larger. An empty content is a file with no bytes: no body on the
 * channel, then the finish.
 */
export type Content = Uint8Array | AsyncIterable<Uint8Array>;

/**
 * Where a file's content comes from: asked once per file the daemon
 * opens a channel for, with the file's path as the request named it,
 * or `null` for the one file of a file upload. `null` back is a file
 * the caller has no content for: the channel is refused with an
 * error, and the daemon abandons that file and answers the scope
 * with its error.
 */
export type Source = (path: string | null) => Content | null;

/**
 * How a content ask is read: the path it names, or `null` for the
 * one file of a file upload, from the channel request's payload. A
 * payload that will not decode throws, and the file is refused.
 */
export type AskDecoder = (payload: Uint8Array) => string | null;

/** Why an upload got no answer: the one-answer kinds, as the Rust's mirror them. */
export type UploadErrorKind = OneShotErrorKind;

/**
 * An upload that did not produce an answer: the open failing; the
 * connection ending first; a frame that could not be read; the
 * daemon finishing the scope without answering; a frame routed here
 * by mistake; or an answer that is not what the endpoint reads. Never
 * the daemon's answer, which `execute` hands back whole — the error
 * a content channel ended in comes back as the daemon's `error`.
 */
export class UploadError extends Error {
  /** Which. */
  readonly kind: UploadErrorKind;

  constructor(kind: UploadErrorKind, cause?: unknown) {
    super(`the upload got no answer: ${kind}`, cause === undefined ? undefined : { cause });
    this.name = "UploadError";
    this.kind = kind;
  }
}

/**
 * Send an upload request, answer every content channel the daemon
 * opens on its scope from `source`, and read the one answer.
 *
 * The content travels on channels the DAEMON opens — only a
 * responder can finish a channel — one per file. Each ask names the
 * file, as the request named it, or nothing for a file upload;
 * `source` is asked for that file's content, which goes out as the
 * channel's responses in pieces of at most `CHUNK_SIZE`, and the
 * channel finishes. A file `source` has no content for, or content
 * that throws partway, sends the channel's own error and finishes
 * it; the daemon abandons that file and answers the scope with its
 * error. The first frame on the scope's response receiver is the
 * answer, whichever the daemon chose, and this returns it; a daemon
 * that answers before asking for anything — a volume it cannot find,
 * forbidden — is read the same way, and no content is sent.
 *
 * Content channels are answered on their own, so files stream at
 * once; one still streaming when the answer arrives is abandoned
 * with the scope. The two receivers are read together: each one's
 * pending read is kept, and only the one that resolved is read
 * again, so no frame is lost to the race.
 */
export async function execute<Q, A>(
  handle: Handle,
  request: Q,
  encode: Encode<Q>,
  decodeAsk: AskDecoder,
  decode: Decode<A>,
  source: Source,
): Promise<A> {
  let scope: Scope;
  try {
    scope = await open(handle, request, encode);
  } catch (error) {
    throw new UploadError(error instanceof OpenError ? error.kind : "send", error);
  }
  let asks = scope.requestReceiver.next();
  let answers = scope.responseReceiver.next();
  for (;;) {
    const arrived = await Promise.race([
      asks.then((bytes) => ({ side: "ask" as const, bytes })),
      answers.then((bytes) => ({ side: "answer" as const, bytes })),
    ]);
    if (arrived.bytes === undefined) {
      throw new UploadError("closed");
    }
    let frame;
    try {
      frame = decodeServerFrame(arrived.bytes);
    } catch (cause) {
      throw new UploadError("frame", cause);
    }
    if (arrived.side === "ask") {
      asks = scope.requestReceiver.next();
      if (frame.kind === "channel_request") {
        void answer(handle, frame.scope, frame.channel, contentFor(decodeAsk, source, frame.payload));
      }
      continue;
    }
    answers = scope.responseReceiver.next();
    switch (frame.kind) {
      case "response":
        try {
          return decode(frame.payload);
        } catch (cause) {
          throw new UploadError("response", cause);
        }
      case "response_finish":
        throw new UploadError("unanswered");
      default:
        throw new UploadError("misrouted");
    }
  }
}

/** The content for one ask: none where the ask will not decode. */
function contentFor(decodeAsk: AskDecoder, source: Source, payload: Uint8Array): Content | null {
  let path: string | null;
  try {
    path = decodeAsk(payload);
  } catch {
    return null;
  }
  return source(path);
}

/**
 * One content channel answered: the pieces, then the finish; or the
 * error, then the finish. A frame the connection would not take ends
 * the answer where it is — the connection is gone, and there is
 * nobody to tell.
 */
async function answer(handle: Handle, scope: number, channel: number, content: Content | null): Promise<void> {
  const delivered =
    content === null
      ? await refuse(handle, scope, channel, "no content for the file asked for")
      : await send(handle, scope, channel, content);
  if (!delivered) {
    return;
  }
  try {
    await handle.sendChannelResponseFinish(scope, channel);
  } catch {
    return;
  }
}

/**
 * Every piece of `content`, each sliced to `CHUNK_SIZE`, as the
 * channel's bodies; content that throws partway is the channel's
 * error instead. Whether the last frame was taken.
 */
async function send(handle: Handle, scope: number, channel: number, content: Content): Promise<boolean> {
  try {
    for await (const piece of content instanceof Uint8Array ? [content] : content) {
      for (let at = 0; at < piece.length; at += CHUNK_SIZE) {
        const body = piece.subarray(at, Math.min(at + CHUNK_SIZE, piece.length));
        if (!(await deliver(handle, scope, channel, channel_response.encode({ body })))) {
          return false;
        }
      }
    }
  } catch (error) {
    return refuse(handle, scope, channel, error instanceof Error ? error.message : String(error));
  }
  return true;
}

/** The channel's own error: the daemon abandons the file. Whether it was taken. */
function refuse(handle: Handle, scope: number, channel: number, message: string): Promise<boolean> {
  return deliver(handle, scope, channel, channel_response.encode({ error: message }));
}

/** One channel response, and whether the connection took it. */
async function deliver(handle: Handle, scope: number, channel: number, payload: Uint8Array): Promise<boolean> {
  try {
    await handle.sendChannelResponse(scope, channel, payload);
    return true;
  } catch {
    return false;
  }
}
