// One item of a log, of seven kinds.

import { z } from "zod";
import { AgenticLoopChunk } from "../../provider/endpoints/containers/agents/run/server/response/chunk.js";
import { Active } from "./active.js";
import { Deployed } from "./deployed.js";
import { Error } from "./error.js";
import { Inactive } from "./inactive.js";
import { Undeployed } from "./undeployed.js";
import { User } from "./user.js";

/**
 * One item of a log: a user part with its sender, a chunk of the
 * loop, an error, or one step of the run's life — `deployed`,
 * `active`, `inactive`, `undeployed`, as `lifecycle` states. Untagged
 * JSON, told apart by each kind's own `type` and, for a user part,
 * by the presence of `sender`: nothing but a user part has one, so
 * `User` is tried first.
 */
export const Item = z.union([User, AgenticLoopChunk, Error, Deployed, Active, Inactive, Undeployed]);
export type Item = z.infer<typeof Item>;
