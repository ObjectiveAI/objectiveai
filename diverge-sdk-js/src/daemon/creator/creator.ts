// Who made a thing.

import { z } from "zod";
import { Agent } from "./agent.js";
import { Client } from "./client.js";
import { Dependency, Record } from "./tool.js";

/**
 * Who made a thing: the client, or an agent or a tool of the
 * client's. JSON-tagged by `type`, `client`, `agent` or `tool`,
 * beside the variant's own members — a tool's own external tag,
 * `record` or `dependency`, beside the `type`.
 */
export const Creator = z.union([
  Client.extend({ type: z.literal("client") }),
  Agent.extend({ type: z.literal("agent") }),
  z.object({ type: z.literal("tool"), record: Record }),
  z.object({ type: z.literal("tool"), dependency: Dependency }),
]);
export type Creator = z.infer<typeof Creator>;
