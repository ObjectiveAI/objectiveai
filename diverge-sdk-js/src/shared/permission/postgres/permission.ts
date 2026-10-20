// One grant's worth over the database.

import { z } from "zod";
import { Action } from "./action.js";

/**
 * The actions held over the database, a bare array on the wire:
 * `["get","list"]`. There is one of it, so there is no `within`; an
 * action is held or it is not.
 */
export const Permission = z.array(Action);
export type Permission = z.infer<typeof Permission>;
