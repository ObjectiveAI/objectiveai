// What brings daemons into being.

import { z } from "zod";

/**
 * The actions over daemons that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"add"`.
 *
 * - `add`: Add a daemon to connect to, as `providers.daemons.add` does.
 */
export const Make = z.enum(["add"]);
export type Make = z.infer<typeof Make>;
