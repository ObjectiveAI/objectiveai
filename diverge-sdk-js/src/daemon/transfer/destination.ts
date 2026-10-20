// The four places a transfer lands.

import { z } from "zod";
import { Agent } from "../reference/agent.js";
import { Dependency } from "../reference/dependency.js";
import { Tool } from "../reference/tool.js";
import { Volume } from "../reference/volume.js";

/**
 * Where a transfer puts what it copies: at a path in an agent's
 * container, a tool's, a dependency tool's, or a volume. Untagged
 * JSON, told apart by its members, and an object with members of
 * more than one does not decode. A file source lands at `path`,
 * replaced whole; a directory source lands as `path`, each file of
 * it at `path` joined with the file's path within the source.
 */
export const Destination = z.union([
  z.strictObject({ agent: Agent, path: z.array(z.string()) }),
  z.strictObject({ tool: Tool, path: z.array(z.string()) }),
  z.strictObject({ dependency: Dependency, path: z.array(z.string()) }),
  z.strictObject({ volume: Volume, path: z.array(z.string()) }),
]);
export type Destination = z.infer<typeof Destination>;
