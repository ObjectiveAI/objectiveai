// What brings roles into being.

import { z } from "zod";

/**
 * The actions over roles that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`.
 *
 * - `create`: Create a role, as `roles.create` does.
 */
export const Make = z.enum(["create"]);
export type Make = z.infer<typeof Make>;
