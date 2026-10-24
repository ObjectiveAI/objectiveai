// What brings tools into being.

import { z } from "zod";

/**
 * The actions over tools that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`, `"register"`.
 *
 * - `create`: Create a tool, as `standalone_tools.create` does.
 * - `register`: Hold another daemon's tool under a name, as `standalone_tools.register` does.
 */
export const Make = z.enum(["create", "register"]);
export type Make = z.infer<typeof Make>;
