// The auth frames' payloads: what each end says in the handshake.
//
// Three frames of type `0`, in order: the connector's offer, the
// connectee's answer, and the connector's proof. A client is the
// connector, so it writes the first and the third and reads the
// second.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/** The length of a public key, in bytes: a point of the curve P-256 in the uncompressed form — the byte `4`, the 32 bytes of `x`, the 32 bytes of `y`. */
export const POINT_LEN = 65;

/** The length of a signature, in bytes: an ECDSA signature over P-256, the 32 bytes of `r` then the 32 bytes of `s`. */
export const SIGNATURE_LEN = 64;

/** The length of a key's thumbprint, in bytes: an RFC 7638 SHA-256 thumbprint. */
export const THUMBPRINT_LEN = 32;

/**
 * The payload of the connector's first auth frame, the first frame of
 * a connection: its one-time P-256 public key, `POINT_LEN` bytes; the
 * public key its token is bound to, `POINT_LEN` bytes; then its token,
 * an RFC 7519 JWT as UTF-8, every byte after them — no length, since
 * WebSocket delimits the message.
 */
export function offer(exchange: Uint8Array, key: Uint8Array, token: string): Uint8Array {
  if (exchange.length !== POINT_LEN || key.length !== POINT_LEN) {
    throw new FrameError("auth");
  }
  const body = utf8(token);
  const out = new Uint8Array(2 * POINT_LEN + body.length);
  out.set(exchange, 0);
  out.set(key, POINT_LEN);
  out.set(body, 2 * POINT_LEN);
  return out;
}

/** What the connectee's one auth frame carries. */
export interface Answer {
  /** The connectee's one-time P-256 public key. */
  readonly exchange: Uint8Array;
  /** The public key the connectee's token is bound to. */
  readonly key: Uint8Array;
  /** The connectee's ECDSA signature over the transcript. */
  readonly signature: Uint8Array;
  /** The connectee's token. */
  readonly token: string;
}

/**
 * The connectee's auth frame's payload, read: its one-time key,
 * `POINT_LEN` bytes; its identity key, `POINT_LEN` bytes; its
 * signature, `SIGNATURE_LEN` bytes; then its token, every byte after
 * them. A payload shorter than the two keys and the signature, one
 * with no byte after them, and a token that is not UTF-8 are
 * malformed: the `auth` error, and the reader closes the connection.
 */
export function readAnswer(payload: Uint8Array): Answer {
  const fixed = 2 * POINT_LEN + SIGNATURE_LEN;
  if (payload.length <= fixed) {
    throw new FrameError("auth");
  }
  try {
    return {
      exchange: payload.subarray(0, POINT_LEN),
      key: payload.subarray(POINT_LEN, 2 * POINT_LEN),
      signature: payload.subarray(2 * POINT_LEN, fixed),
      token: text(payload.subarray(fixed)),
    };
  } catch (cause) {
    throw new FrameError("auth", undefined, cause);
  }
}

/**
 * The payload of the connector's second auth frame, the handshake's
 * last: its ECDSA signature over the transcript, exactly
 * `SIGNATURE_LEN` bytes, and nothing else.
 */
export function proof(signature: Uint8Array): Uint8Array {
  if (signature.length !== SIGNATURE_LEN) {
    throw new FrameError("auth");
  }
  return signature;
}
