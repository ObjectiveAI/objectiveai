// One connected tool of the caller's, named either way.

import { z } from "zod";
import { DaemonId } from "../endpoints/daemons/id.js";
import { Tool } from "./tool.js";

/**
 * One connected tool of the caller's — another daemon's standalone
 * tool held here: by its name, or by the daemon it is on, by the ID
 * of this daemon's record of it, the tool as that daemon names it,
 * and its index among the connected tools held from that tool.
 * Untagged JSON, one object either way; an object with members of
 * both does not decode.
 */
export const ConnectedStandaloneTool = z.union([
  /** By name: `{"name":…}`. A connected tool given none is not reached this way. */
  z.strictObject({ name: z.string() }),
  /** By daemon, tool and index: `{"daemon":…,"tool":…,"index":…}`. */
  z.strictObject({ daemon: DaemonId, tool: Tool, index: z.number().int().nonnegative() }),
]);
export type ConnectedStandaloneTool = z.infer<typeof ConnectedStandaloneTool>;
