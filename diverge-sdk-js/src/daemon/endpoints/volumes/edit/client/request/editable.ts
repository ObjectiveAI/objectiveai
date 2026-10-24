// The volume as a patch sees it.

import { z } from "zod";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";

/**
 * The members of a volume an edit may change, as one JSON document:
 * its size and its mode, as the provider lists them, and as the
 * result must be again. The name does not change.
 */
export const Editable = z.strictObject({
  /** How many bytes it reserves: larger reserves more, smaller gives room back and is refused when the volume holds more than that. */
  bytes: z.number().int().nonnegative(),
  /** The mode it is in. The content is untouched either way. */
  mode: Mode,
});
export type Editable = z.infer<typeof Editable>;
