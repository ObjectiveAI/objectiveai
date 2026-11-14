// A volumes fuse serve's first answer.

import { z } from "zod";
import { DecodeError, jsonOf, splitTag } from "../../../../../../wire/decode.js";
import { VolumeMode } from "../../../../../../shared/containers/response/volume_mode.js";
import { Error } from "../../../../../../shared/error.js";

/**
 * Whether the volume is served: exactly one, first on the main
 * stream. After `serving` the scope stays open and nothing more comes
 * on the main stream until the finish; after any other the finish
 * follows at once, and no channel is answered. `mounted`,
 * `fuse_served`, `held`, `volume_mode` and `unsupported` are the
 * provider's own refusals, relayed as it gave them.
 */
export const Frame = z.union([
  /** The provider serves the volume, and the scope carries its asks. Tag `0`. */
  z.literal("serving"),
  /** A running container has the volume, and the provider serves no volume of its mode beside one. Tag `1`. */
  z.literal("mounted"),
  /** Another serve holds the volume, and the provider serves a volume of its mode once. Tag `2`. */
  z.literal("fuse_served"),
  /** Another operation has the volume to itself. Tag `3`. */
  z.literal("held"),
  /** The volume is in another mode than the request meant: its name, and the mode it is in. Tag `4`. */
  z.strictObject({ volume_mode: VolumeMode }),
  /** The provider serves no volume of that mode. Tag `5`. */
  z.literal("unsupported"),
  /** The account the request is served for holds no `fuse_serve` grant reaching the volume; nothing was asked of the provider. Tag `6`. */
  z.literal("forbidden"),
  /** The provider is none the daemon keeps a record of, or the volume is none the provider lists. Tag `7`. */
  z.literal("not_found"),
  /** A failure, in the daemon's words or the provider's. Tag `8`. */
  z.strictObject({ error: Error }),
]);
export type Frame = z.infer<typeof Frame>;

const SERVING = 0;
const MOUNTED = 1;
const FUSE_SERVED = 2;
const HELD = 3;
const VOLUME_MODE = 4;
const UNSUPPORTED = 5;
const FORBIDDEN = 6;
const NOT_FOUND = 7;
const ERROR = 8;

/**
 * A frame out of its payload: the tag byte, then — for the mode
 * refusal and the error alone — the variant's value as JSON. A tag
 * that names no variant is the `unexpected_tag` error.
 */
export function decode(bytes: Uint8Array): Frame {
  const [tag, rest] = splitTag(bytes);
  switch (tag) {
    case SERVING:
      return "serving";
    case MOUNTED:
      return "mounted";
    case FUSE_SERVED:
      return "fuse_served";
    case HELD:
      return "held";
    case VOLUME_MODE:
      return { volume_mode: VolumeMode.parse(jsonOf(rest)) };
    case UNSUPPORTED:
      return "unsupported";
    case FORBIDDEN:
      return "forbidden";
    case NOT_FOUND:
      return "not_found";
    case ERROR:
      return { error: Error.parse(jsonOf(rest)) };
    default:
      throw new DecodeError("unexpected_tag", tag);
  }
}
