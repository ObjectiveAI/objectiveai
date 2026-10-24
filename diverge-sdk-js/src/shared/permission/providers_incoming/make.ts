// What brings incoming credentials into being.

import { z } from "zod";

/**
 * The actions over incoming credentials that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"add"`.
 *
 * - `add`: Add an incoming credential, as `providers.incoming.add` does.
 */
export const Make = z.enum(["add"]);
export type Make = z.infer<typeof Make>;
