// Pinning a container to one provider, with its volumes.

import { z } from "zod";
import { Identity } from "../../../../../agent_log/identity.js";
import { VolumeMount } from "./volume_mount.js";

/**
 * Pin an agent, or a tool, to one provider, and mount that
 * provider's volumes in it. Named on the create and never on a
 * template: a template is shareable, and a provider is one daemon's
 * acquaintance. A volume is a provider's own, so a container that
 * mounts volumes can run on one provider only.
 */
export const Provider = z.object({
  /** Which provider. */
  identity: Identity,
  /** Volumes of that provider, made visible inside the container, in order. Absent, none: the pin alone. */
  volume_mounts: z.array(VolumeMount).optional(),
});
export type Provider = z.infer<typeof Provider>;
