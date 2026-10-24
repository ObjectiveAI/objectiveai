// What is done with the events.

import { z } from "zod";

/**
 * The actions over the events, each the endpoint of the same name under `events`.
 * Snake case on the wire: `"list"`.
 *
 * - `list`: List the events, narrowed, and keep the list.
 */
export const Action = z.enum(["list"]);
export type Action = z.infer<typeof Action>;
