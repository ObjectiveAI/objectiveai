// The incoming credential as a patch sees it.

import { z } from "zod";
import { Credential } from "../../../../../../../daemon/endpoints/providers/incoming/credential.js";

/**
 * The members of a credential of incoming providers an edit may
 * change, as one JSON document: the credential without its key — the
 * identity it names and the address it is accepted from — as the
 * daemon lists it, and as the result must be again.
 */
export const Editable = z.strictObject({
  /** The credential as it is to be, without its key. */
  credential: Credential,
});
export type Editable = z.infer<typeof Editable>;
