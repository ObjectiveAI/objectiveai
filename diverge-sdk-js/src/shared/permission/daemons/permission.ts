// A grant over remote daemons.

import { z } from "zod";
import { Action } from "./action.js";

/** The acts a grant allows over every remote daemon this daemon reaches: as JSON, the list of them, `["get","relay"]`. */
export const Permission = z.array(Action);
export type Permission = z.infer<typeof Permission>;
