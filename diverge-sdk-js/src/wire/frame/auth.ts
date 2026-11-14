// The auth frames' payloads: what each end says in the handshake.
//
// Three frames of type `0`, in order: the connector's offer, the
// connectee's answer, and the connector's proof. A client is the
// connector, so it writes the first and the third and reads the
// second.

import { utf8 } from "../encode.js";
import { text } from "../decode.js";
import { FrameError } from "./error.js";

/** The length of a public key, in bytes: an X25519 one-time key, or an Ed25519 key. */
export const KEY_LEN = 32;

/** The length of a signature, in bytes: an Ed25519 one. */
export const SIGNATURE_LEN = 64;

/**
 * The payload of the connector's first auth frame, the first frame of
 * a connection: its one-time X25519 public key, `KEY_LEN` bytes, then
 * its token, an RFC 7519 JWT as UTF-8, every byte after the key — no
 * length, since WebSocket delimits the message.
 */
export function offer(key: Uint8Array, token: string): Uint8Array {
  const body = utf8(token);
  const out = new Uint8Array(KEY_LEN + body.length);
  out.set(key, 0);
  out.set(body, KEY_LEN);
  return out;
}

/** What the connectee's one auth frame carries. */
export interface Answer {
  /** The connectee's one-time X25519 public key. */
  readonly key: Uint8Array;
  /** The connectee's Ed25519 signature over the transcript. */
  readonly signature: Uint8Array;
  /** The connectee's token. */
  readonly token: string;
}

/**
 * The connectee's auth frame's payload, read: its one-time key,
 * `KEY_LEN` bytes; its signature, `SIGNATURE_LEN` bytes; then its
 * token, every byte after them. A payload shorter than the key and
 * the signature, one with no byte after them, and a token that is not
 * UTF-8 are malformed: the `auth` error, and the reader closes the
 * connection.
 */
export function readAnswer(payload: Uint8Array): Answer {
  if (payload.length <= KEY_LEN + SIGNATURE_LEN) {
    throw new FrameError("auth");
  }
  try {
    return {
      key: payload.subarray(0, KEY_LEN),
      signature: payload.subarray(KEY_LEN, KEY_LEN + SIGNATURE_LEN),
      token: text(payload.subarray(KEY_LEN + SIGNATURE_LEN)),
    };
  } catch (cause) {
    throw new FrameError("auth", undefined, cause);
  }
}

/**
 * The payload of the connector's second auth frame, the handshake's
 * last: its Ed25519 signature over the transcript, exactly
 * `SIGNATURE_LEN` bytes, and nothing else.
 */
export function proof(signature: Uint8Array): Uint8Array {
  if (signature.length !== SIGNATURE_LEN) {
    throw new FrameError("auth");
  }
  return signature;
}
