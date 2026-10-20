// The outgoing provider as a patch sees it.

import { z } from "zod";
import { Mode } from "../../../../../../../daemon/endpoints/providers/outgoing/mode.js";

/**
 * The members of an outgoing provider an edit may change, as one
 * JSON document: the mode, presented as `null` — the daemon never
 * answers a credential — and required in the result: a result with
 * `null` is the edit's error. The address does not change.
 */
export const Editable = z.strictObject({
  /** How to authenticate there from now on. `null` as presented; given whole by the patch. */
  mode: Mode.nullable(),
});
export type Editable = z.infer<typeof Editable>;
