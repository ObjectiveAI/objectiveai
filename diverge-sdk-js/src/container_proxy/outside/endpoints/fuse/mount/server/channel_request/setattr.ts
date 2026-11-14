// The ask to change a path's attributes.

import { z } from "zod";
import { concat, text } from "../../../../../../../shared/containers/fuse/bytes.js";
import * as attrs from "../../../../../../../shared/containers/fuse/setattr/attrs.js";

/**
 * A path's attributes to change: which of its mode, owner, group and
 * times, and to what. The attributes' bytes, then the path, to the
 * end of the frame.
 */
export const Setattr = z.object({
  /** The path. */
  path: z.string(),
  /** What to change: each attribute present is changed. */
  attrs: attrs.Attrs,
});
export type Setattr = z.infer<typeof Setattr>;

/** The ask as its bytes. */
export function encode(ask: Setattr): Uint8Array {
  return concat(attrs.encode(ask.attrs), new TextEncoder().encode(ask.path));
}

/** An ask out of its bytes. */
export function decode(bytes: Uint8Array): Setattr {
  const [changed, path] = attrs.decode(bytes);
  return { path: text(path), attrs: changed };
}
