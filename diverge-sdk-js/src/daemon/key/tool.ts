// One tool, once and for all.

import { z } from "zod";
import { DaemonId } from "../endpoints/daemons/id.js";
import { Tool as Reference } from "../reference/tool.js";
import { Owner } from "./owner.js";

/**
 * One tool of the caller's, once and for all: a standalone tool on
 * record, by its template and its index; a connected tool, by the
 * daemon it is on, the tool as that daemon names it and its index —
 * each with its name beside when it has one; or a dependency tool, by
 * the template it runs from and whose it is. Externally tagged:
 * `{"record":{…}}`, `{"connected_standalone":{…}}` or
 * `{"dependency":{…}}`.
 */
export const Tool = z.union([
  z.strictObject({
    record: z.object({
      /** The template it was made from, by id. */
      template: z.string(),
      /** Its number among all tools of the caller's ever made from that template. */
      index: z.number().int().nonnegative(),
      /** Its name, if it has one. */
      name: z.string().optional(),
    }),
  }),
  z.strictObject({
    connected_standalone: z.object({
      /** The daemon it is on, by the ID of this daemon's record of it. */
      daemon: DaemonId,
      /** The tool, as that daemon names it. */
      tool: Reference,
      /** Its number among all connected tools of the caller's ever held from that tool. */
      index: z.number().int().nonnegative(),
      /** Its name, if it has one. */
      name: z.string().optional(),
    }),
  }),
  z.strictObject({
    dependency: z.object({
      /** The dependency template it runs from, by id. */
      template: z.string(),
      /** Whose it is. */
      owner: Owner,
    }),
  }),
]);
export type Tool = z.infer<typeof Tool>;
