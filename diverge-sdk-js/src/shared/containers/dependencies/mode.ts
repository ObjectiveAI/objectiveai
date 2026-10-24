// Which of the two modes a dependency template declares.

import { z } from "zod";
import { Mount } from "./mount.js";
import { Scope } from "./scope.js";

/** A tool for the agent's run: `"mode":"transient"`, with the agent's paths it mounts. */
export const Transient = z.object({
  mode: z.literal("transient"),
  /** Files of the agent container served live into the tool's, one mount each. Absent when none. */
  fuse_file_mounts: z.array(Mount).optional(),
  /** Directories of the agent container served live into the tool's, one mount each. Absent when none. */
  fuse_directory_mounts: z.array(Mount).optional(),
});
export type Transient = z.infer<typeof Transient>;

/** A service the caller keeps: `"mode":"service"`, with how far it is shared. */
export const Service = z.object({
  mode: z.literal("service"),
  /** How far the service is shared: see `Scope`. */
  scope: Scope,
});
export type Service = z.infer<typeof Service>;

/**
 * A transient tool, or a service: the two things a dependency can
 * be, told apart on the wire by the `mode` member beside the
 * template's other members, with the members of that mode flat
 * beside it. A transient tool is deployed when the agent's container
 * starts and stopped with the agent's run; a service is made once,
 * under the scope key its `Scope` names, and kept by the caller for
 * as long as any agent is attached to it.
 */
export const Mode = z.discriminatedUnion("mode", [Transient, Service]);
export type Mode = z.infer<typeof Mode>;
