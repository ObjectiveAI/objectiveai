// The channels a caller opens on a FUSE serve scope.

import * as mount from "../../../../../../container_proxy/outside/endpoints/fuse/mount/server/channel_request/frame.js";
import { FuseError } from "../../../../../../shared/containers/fuse/error.js";

/**
 * What a caller asks for on a served scope: one of the nine asks a
 * FUSE mount makes, with no mount id, since the scope is the volume;
 * the cancel that ends the scope; or the file tree.
 *
 * | tag | asks for |
 * |-----|----------|
 * | `0`–`8` | an `ask`: one of the nine, under its own tag; one answer, then the finish |
 * | `9` | `cancel`: carries nothing, and nothing answers on its channel; the scope's finish does |
 * | `10` | `filetree`: carries nothing; the tree as the serve sees it, then one frame per change, for the scope's life |
 */
export type Frame = { readonly ask: mount.Frame } | "cancel" | "filetree";

const CANCEL = 9;
const FILETREE = 10;

/** The channel request as its bytes: an ask as the ask's own, the others their one tag byte. */
export function encode(frame: Frame): Uint8Array {
  if (frame === "cancel") {
    return Uint8Array.of(CANCEL);
  }
  if (frame === "filetree") {
    return Uint8Array.of(FILETREE);
  }
  return mount.encode(frame.ask);
}

/** A channel request out of its bytes. */
export function decode(bytes: Uint8Array): Frame {
  if (bytes.length === 0) {
    throw new FuseError("empty");
  }
  switch (bytes[0]) {
    case CANCEL:
      return "cancel";
    case FILETREE:
      return "filetree";
    default:
      return { ask: mount.decode(bytes) };
  }
}
