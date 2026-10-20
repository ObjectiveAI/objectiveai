// What brings tool templates into being.

import { z } from "zod";

/**
 * The actions over tool templates that make one where there was none, which a grant holds or does not and judges by nothing else.
 * Snake case on the wire: `"create"`.
 *
 * - `create`: Make a tool template, as `standalone_tools.templates.create` does.
 */
export const Make = z.enum(["create"]);
export type Make = z.infer<typeof Make>;
