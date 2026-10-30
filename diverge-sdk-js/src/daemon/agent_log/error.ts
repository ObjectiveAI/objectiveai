// An error the run met, as the log keeps it.

import { z } from "zod";
import * as shared from "../../shared/error.js";

/**
 * One error the log kept: `type` is the string `error`, and the error
 * itself — the run that would not start, or the message the agent
 * refused, in the words the daemon received — is whole under `error`.
 *
 * The error is an arbitrary JSON value, as `shared::error::Error` says;
 * it is kept under its own member rather than flattened so that nothing
 * in it can be mistaken for the item's own `type`.
 */
export const Error = z.object({
  /** Which kind this is. */
  type: z.literal("error"),
  /** The failure, in the daemon's or the provider's own words. */
  error: shared.Error,
});
export type Error = z.infer<typeof Error>;
