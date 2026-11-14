// What brings a connected tool into being.

import { z } from "zod";

/**
 * The actions over connected tools that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`.
 *
 * - `create`: Hold another daemon's standalone tool under a name, as `connected_standalone_tools.create` does, the daemon it is on judged by a `relay` grant over daemons.
 */
export const Make = z.enum(["create"]);
export type Make = z.infer<typeof Make>;
