// What the daemon tells of a tool it connected.

import { z } from "zod";
import { Image } from "../../../../../../daemon/endpoints/agents/create/client/request/image.js";

/**
 * What the connecting side cannot see for itself: the image the
 * tool's container was made from, and the template it was made
 * from, by id — so that a daemon connecting on behalf of its own
 * agents can attest both under `_meta` as its own `connected` stamp.
 */
export const Connected = z.object({
  /** The image the container runs. */
  image: Image,
  /** The template the tool was made from, by id. */
  template: z.string(),
});
export type Connected = z.infer<typeof Connected>;
