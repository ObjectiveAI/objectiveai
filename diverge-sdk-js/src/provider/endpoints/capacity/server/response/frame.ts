// What a server's response frame carries for a capacity request.

import { z } from "zod";

/**
 * How much the caller may still use: four counts of bytes, each the
 * most of its resource the caller could use up as of the instant of
 * the answer, as the provider allots it. Nothing is set aside by
 * answering. Every member is sent; a caller reads what the
 * capabilities allow — `volumes_disk` only where a volume mode is
 * held out, `images_disk` only where `images_store` is. On the
 * provider's wire the struct as JSON and nothing in front of it.
 */
export const Frame = z.looseObject({
  /** The largest size, in bytes, a new volume of the caller could have. */
  volumes_disk: z.number().int().nonnegative(),
  /** The bytes the caller could still add to the client image store by a build. */
  images_disk: z.number().int().nonnegative(),
  /** The bytes of memory a container run of the caller could still reserve. */
  container_memory: z.number().int().nonnegative(),
  /** The bytes of overlay disk a container run of the caller could still reserve. */
  container_disk: z.number().int().nonnegative(),
});
export type Frame = z.infer<typeof Frame>;
