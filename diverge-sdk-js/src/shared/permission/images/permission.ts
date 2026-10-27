// One grant's worth over images.

import { z } from "zod";
import { Action } from "./action.js";

/**
 * The actions held over images, a bare array on the wire: `["info"]`.
 * An image is named by its digest and is no record the daemon keeps,
 * so there is no `within`; an action is held or it is not.
 */
export const Permission = z.array(Action);
export type Permission = z.infer<typeof Permission>;
