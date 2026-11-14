// A run, or a FUSE serve, refused because a volume is not in the mode it meant.

import { z } from "zod";
import { Mode } from "../../../provider/endpoints/volumes/mode.js";

/**
 * The volume a request named under one mode that is in another, and
 * so refused the request. Nothing changed: the volume is in the mode
 * reported here, and only a volumes edit changes it.
 */
export const VolumeMode = z.strictObject({
  /** The name of the volume, as the request named it. */
  name: z.string(),
  /** The mode the volume is in. */
  mode: Mode,
});
export type VolumeMode = z.infer<typeof VolumeMode>;
