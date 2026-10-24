// What a client's channel response frame carries on a write's
// content channel.

import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { json, tagged } from "../../../../../../wire/encode.js";
import { Error } from "../../../../../../shared/error.js";
import * as write_bytes from "../../../../../../shared/containers/write_bytes/response/frame.js";

/**
 * A piece of the file being written, or the news that there will not
 * be one. A payload leads with one byte saying which — `0` for
 * `body`, `1` for `error` — and the rest is that variant's own
 * bytes: the piece raw, or the error's JSON.
 *
 * The channel ends with bodies, then a finish — that was the whole
 * content — or with an `error`, then a finish — the full content was
 * not streamed. Only a finish ends it. A write whose content ended
 * in an error is abandoned: the server discards what it wrote, and
 * answers the scope with an error of its own. Each body is at most
 * `CHUNK_SIZE`, appended to the pieces before it; the error beside
 * it is the exchange's own — the client cannot supply the content it
 * was asked for: its source went away, the read it was piping
 * stopped, whatever it knows.
 */
export type Frame = { readonly body: write_bytes.Frame } | { readonly error: Error };

/** Tag for `body`. */
const BODY = 0;
/** Tag for `error`. */
const ERROR = 1;

/** The tag, then the piece as it is, or the error as JSON. */
export function encode(frame: Frame): Uint8Array {
  return "body" in frame ? tagged(BODY, write_bytes.encode(frame.body)) : tagged(ERROR, json(frame.error));
}

/**
 * A frame out of its payload: the tag byte, then the piece raw or
 * the error's JSON. A tag that names no variant is the
 * `unexpected_tag` error; no bytes at all is `empty`, which is not a
 * zero-length body — a tag followed by nothing, ordinary on a stream.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case BODY:
      return { body: write_bytes.decode(rest) };
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
