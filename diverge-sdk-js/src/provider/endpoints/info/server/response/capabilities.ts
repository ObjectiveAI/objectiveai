// What a provider does, of everything the specification lets a
// provider do.

import { z } from "zod";
import { VolumeCapabilities, kept } from "./volume_capabilities.js";

/**
 * Which of the specification's optional parts this provider serves.
 * Every provider serves every endpoint's wire, but not every
 * provider does everything a request may ask: one has no image
 * store, one pulls from no registry, one keeps volumes of some modes
 * only. Each such part is one member here, and a request past it is
 * not a failure but a refusal the endpoint has a variant of its own
 * for. Flat, and tolerant: a member absent is the capability absent;
 * a member a reader does not know is ignored.
 */
export const Capabilities = z.looseObject({
  /** The provider keeps an image store for its callers. Absent, `false`. */
  images_store: z.boolean().optional(),
  /** The registries the provider pulls from, each a host. Absent, none. */
  images_registries: z.array(z.string()).optional(),
  /** What may be done with a volume in the `persistent` mode. Absent, the mode is not kept. */
  volumes_persistent: VolumeCapabilities.optional(),
  /** What may be done with a volume in the `ephemeral` mode. */
  volumes_ephemeral: VolumeCapabilities.optional(),
  /** What may be done with a volume in the `read_only` mode. */
  volumes_read_only: VolumeCapabilities.optional(),
});
export type Capabilities = z.infer<typeof Capabilities>;

/** Whether the provider keeps volumes at all: any mode among the three. */
export function volumes(capabilities: Capabilities): boolean {
  return [capabilities.volumes_persistent, capabilities.volumes_ephemeral, capabilities.volumes_read_only].some(
    (mode) => mode !== undefined && kept(mode),
  );
}
