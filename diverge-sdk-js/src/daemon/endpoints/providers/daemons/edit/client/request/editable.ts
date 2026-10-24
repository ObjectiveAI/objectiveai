// The daemon record as a patch sees it.

import { z } from "zod";
import { Link } from "../../../../../../../daemon/endpoints/providers/daemons/link.js";

/**
 * The members of a daemon record an edit may change, as one JSON
 * document: the links, as the daemon lists them, and as the result
 * must be again. The name does not change, and the accounts are each
 * their own edit's.
 */
export const Editable = z.strictObject({
  /** The providers it is reached through, in order. */
  links: z.array(Link),
});
export type Editable = z.infer<typeof Editable>;
