// A grant over providers.

import { z } from "zod";
import { Action } from "./action.js";

/** The acts a grant allows over every provider of the daemon's: as JSON, the list of them, `["get","list"]`. */
export const Permission = z.array(Action);
export type Permission = z.infer<typeof Permission>;
