// Which account a request means.

import { z } from "zod";

/**
 * Which account a request means: by its name, or by the identity
 * its credential names. Untagged JSON, one object either way,
 * `{"name":…}` or `{"identity":…}`; an object with both does not
 * decode. A named account with a credential is named by either.
 */
export const Reference = z.union([z.strictObject({ name: z.string() }), z.strictObject({ identity: z.string() })]);
export type Reference = z.infer<typeof Reference>;
