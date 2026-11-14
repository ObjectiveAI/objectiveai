// Where a provider took an image from.

import { z } from "zod";

/**
 * Where a provider took an image from: a registry it uses, the
 * caller, who proved it holds it, or the caller's own Store. On the
 * wire `{"registry":{"reference":"…"}}`, `"client"` or `"store"`. A
 * push never answers `store`: a digest owned already is `exists`.
 */
export const Source = z.union([
  /** A registry the provider uses served the image: the reference it took it by, `<registry>/<name>@<digest>`. */
  z.strictObject({ registry: z.object({ reference: z.string() }) }),
  /** The caller supplied the image, and proved that it holds it. */
  z.literal("client"),
  /** The caller's own Store held the image. A run's answer only. */
  z.literal("store"),
]);
export type Source = z.infer<typeof Source>;
