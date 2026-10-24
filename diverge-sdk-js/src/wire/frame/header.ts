// The nine-byte header, split off and put on.

import { FrameError } from "./error.js";

/** The header's length: one type byte and two big-endian `u32`s. */
export const HEADER_LEN = 1 + 4 + 4;

/** A frame taken apart: its header's three fields and the rest. */
export interface Header {
  /** The frame type, one of the seven. */
  type: number;
  /** The scope the frame belongs to. */
  scope: number;
  /** The channel inside it; `0` for the scope's own answer. */
  channel: number;
  /** Everything after the header: the payload, as given. */
  payload: Uint8Array;
}

/**
 * Split one frame into its header and payload. A frame shorter than
 * the header is `truncated`, which is the one envelope error a
 * frame can have besides a type nobody defines.
 */
export function splitHeader(bytes: Uint8Array): Header {
  if (bytes.length < HEADER_LEN) {
    throw new FrameError("truncated");
  }
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  return {
    type: view.getUint8(0),
    scope: view.getUint32(1, false),
    channel: view.getUint32(5, false),
    payload: bytes.subarray(HEADER_LEN),
  };
}

/**
 * One frame: the header written in front of `payload`, one
 * allocation. A frame carrying nothing is the header alone.
 */
export function joinHeader(type: number, scope: number, channel: number, payload: Uint8Array = EMPTY): Uint8Array {
  const out = new Uint8Array(HEADER_LEN + payload.length);
  const view = new DataView(out.buffer);
  view.setUint8(0, type);
  view.setUint32(1, scope, false);
  view.setUint32(5, channel, false);
  out.set(payload, HEADER_LEN);
  return out;
}

const EMPTY = new Uint8Array(0);
