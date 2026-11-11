// What records a remote daemon.

import { z } from "zod";

/**
 * The actions over remote daemons that make a record where there was none.
 * Snake case on the wire: `"add"`.
 *
 * - `add`: Record a remote daemon, as `daemons.add` does.
 */
export const Make = z.enum(["add"]);
export type Make = z.infer<typeof Make>;
