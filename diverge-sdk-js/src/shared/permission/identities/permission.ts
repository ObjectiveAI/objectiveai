// A grant over the host's identities.

import { z } from "zod";
import { Action } from "./action.js";

/** The acts a grant allows over every identity of the host's: as JSON, the list of them, `["issuers","subjects"]`. */
export const Permission = z.array(Action);
export type Permission = z.infer<typeof Permission>;
