// The directory a build reads from.

import { z } from "zod";

/**
 * A directory of one volume on the build's provider, as the context
 * of the build: what the Containerfile's `COPY` and `ADD` resolve
 * against. The build reads it and writes nothing into it. The volume
 * is on the provider the request names, since a build reads where it
 * runs. The path is components into the volume: no component means
 * "up", so a context cannot reach outside its volume.
 */
export const Context = z.strictObject({
  /** The volume, by its name on the build's provider. */
  volume: z.string(),
  /** How far into the volume the context is, as path components. Absent or empty, the volume itself. */
  path: z.array(z.string()).optional(),
});
export type Context = z.infer<typeof Context>;
