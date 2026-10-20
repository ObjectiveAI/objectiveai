// What brings accounts into being.

import { z } from "zod";

/**
 * The actions over accounts that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`.
 *
 * - `create`: Create an account, as `accounts.create` does.
 */
export const Make = z.enum(["create"]);
export type Make = z.infer<typeof Make>;
