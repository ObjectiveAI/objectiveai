// What may be done to the host's identities.

import { z } from "zod";

/**
 * An act over the host's identities.
 * Snake case on the wire: `"issuers"`, `"subjects"`.
 *
 * - `issuers`: List the issuers an identity is signed in at, as `identities.issuers` does.
 * - `subjects`: List the subjects signed in at one issuer, as `identities.subjects` does.
 */
export const Action = z.enum(["issuers", "subjects"]);
export type Action = z.infer<typeof Action>;
