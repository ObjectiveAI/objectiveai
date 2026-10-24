// What brings agent templates into being.

import { z } from "zod";

/**
 * The actions over agent templates that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`.
 *
 * - `create`: Make a template, as `agents.templates.create` does.
 */
export const Make = z.enum(["create"]);
export type Make = z.infer<typeof Make>;
