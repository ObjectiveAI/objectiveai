// The channels a client opens on a terminal.

import { json, tagged } from "../../../../../../wire/encode.js";
import type { Open } from "../../../../../../shared/containers/terminal/open/open.js";

/**
 * What a client asks the daemon for while a terminal is open: `close`,
 * tag `0`, carrying nothing, which kills the shell and is answered by
 * the scope's finish; `stdin`, tag `1`, the bytes as they are, written
 * to the shell's stdin; `resize`, tag `2`, the size as JSON. A
 * `stdin` and a `resize` are each answered by the channel's finish
 * once done, and the next `stdin` is sent once the last is finished,
 * which keeps them in order.
 */
export type Frame = "close" | { readonly stdin: Uint8Array } | { readonly resize: Open };

const CLOSE = 0;
const STDIN = 1;
const RESIZE = 2;

/** One byte of tag, then nothing, the bytes, or JSON. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "close") {
    return Uint8Array.of(CLOSE);
  }
  if ("stdin" in frame) {
    return tagged(STDIN, frame.stdin);
  }
  return tagged(RESIZE, json(frame.resize));
}
