// A volume of the pinned provider, mounted into the container.

import { z } from "zod";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";

/**
 * A volume of the provider the container is pinned to, mounted into
 * it: host side first — the volume by name, and how far into it to
 * start — then the container side. A caller cannot escape upward:
 * the offset is components, and `..` is a name, not an instruction.
 * What becomes of the changes is the volume's own mode; a mount only
 * states which mode it means the volume to have.
 */
export const VolumeMount = z.object({
  /** Which volume, by the name the provider's listing gives it. Naming it takes the `mount` grant over it. */
  volume_name: z.string(),
  /** How far into that volume to start, as path components relative to it. Empty mounts the volume itself. */
  volume_relative_path: z.array(z.string()),
  /** The mode the volume is meant to be in; a volume in another mode refuses the run. */
  volume_mode: Mode,
  /** Where it appears inside the container, as path components from the root. Empty is refused. */
  container_path: z.array(z.string()),
});
export type VolumeMount = z.infer<typeof VolumeMount>;
