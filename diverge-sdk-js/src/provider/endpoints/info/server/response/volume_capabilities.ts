// What may be done with a volume of one mode, on one provider.

import { z } from "zod";

/**
 * What a provider does with a volume of one mode: whether a run may
 * mount it, whether a FUSE serve may hold it, and how many of each,
 * and both, at once. Each member a boolean, absent being `false`;
 * every member `false` — the object absent — is a mode the provider
 * keeps no volume in. The later ones extend the earlier: `mount_many`
 * means nothing without `mount`, `fuse_serve_many` nothing without
 * `fuse_serve`, `mount_and_fuse_serve` nothing without both.
 */
export const VolumeCapabilities = z.object({
  /** A run may mount a volume of the mode. */
  mount: z.boolean().optional(),
  /** A FUSE serve may hold a volume of the mode. */
  fuse_serve: z.boolean().optional(),
  /** More than one running container may mount a volume of the mode at once. */
  mount_many: z.boolean().optional(),
  /** More than one FUSE serve may hold a volume of the mode at once. */
  fuse_serve_many: z.boolean().optional(),
  /** A running container and a FUSE serve may hold a volume of the mode at once. */
  mount_and_fuse_serve: z.boolean().optional(),
});
export type VolumeCapabilities = z.infer<typeof VolumeCapabilities>;

/** Whether the provider keeps volumes of the mode at all: any member `true`. */
export function kept(capabilities: VolumeCapabilities): boolean {
  return Boolean(
    capabilities.mount ||
      capabilities.fuse_serve ||
      capabilities.mount_many ||
      capabilities.fuse_serve_many ||
      capabilities.mount_and_fuse_serve,
  );
}
