// What brings an identity onto the host.

import { z } from "zod";

/**
 * The actions over identities that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"login"`.
 *
 * - `login`: Sign in an identity not on the host yet, as `identities.login` does.
 */
export const Make = z.enum(["login"]);
export type Make = z.infer<typeof Make>;
