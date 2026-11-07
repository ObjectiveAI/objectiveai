// What records a provider.

import { z } from "zod";

/**
 * The actions over providers that make a record where there was none.
 * Snake case on the wire: `"add"`.
 *
 * - `add`: Record a provider, as `providers.add` does.
 */
export const Make = z.enum(["add"]);
export type Make = z.infer<typeof Make>;
