// What brings vault keys into being.

import { z } from "zod";

/**
 * The actions over vault keys that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"insert"`.
 *
 * - `insert`: Insert into a key the vault does not hold, as `vault.insert` does where the key is new; over a key it holds, the same request is `Over`'s `insert`.
 */
export const Make = z.enum(["insert"]);
export type Make = z.infer<typeof Make>;
