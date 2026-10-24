// Who a response came from.

import { z } from "zod";
import { Image } from "../../../daemon/endpoints/agents/create/client/request/image.js";
import { Owner } from "../../../daemon/key/owner.js";

/**
 * Who a response came from, as the daemon attests it on the
 * response's `_meta`: a tool the daemon made, one it connected to
 * another daemon's, or a dependency tool — each with the image it
 * runs. Tagged by `kind`.
 */
export const Tool = z.discriminatedUnion("kind", [
  /** A tool made here, by its template and its index, with its name when it has one. */
  z.object({ kind: z.literal("created"), image: Image, template: z.string(), index: z.number().int().nonnegative(), name: z.string().optional() }),
  /** Another daemon's tool, as served to this one: the daemon it is reached on, by its name in this daemon's configuration. */
  z.object({
    kind: z.literal("connected"),
    image: Image,
    template: z.string(),
    index: z.number().int().nonnegative(),
    name: z.string().optional(),
    daemon: z.string(),
  }),
  /** A dependency tool, by the template it runs from and whose it is. */
  z.object({ kind: z.literal("dependency"), image: Image, template: z.string(), owner: Owner }),
]);
export type Tool = z.infer<typeof Tool>;
