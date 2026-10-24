// What brings outgoing providers into being.

import { z } from "zod";

/**
 * The actions over outgoing providers that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"add"`.
 *
 * - `add`: Add a provider to dial, as `providers.outgoing.add` does.
 */
export const Make = z.enum(["add"]);
export type Make = z.infer<typeof Make>;
