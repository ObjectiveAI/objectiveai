// A file or a directory of any provider's volume, served live.

import { z } from "zod";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";
import { Identity } from "../../../logs/server/response/identity.js";

/**
 * A file or a directory in a volume of some provider the daemon
 * knows, mounted into the container over FUSE: which provider,
 * which of its volumes, where in it, and where it appears. The
 * daemon stands between, forwarding every ask to the volume's
 * provider and every answer back; nothing is copied or buffered. A
 * FILE mount is one regular file that can be read and overwritten
 * in place but never replaced by a rename; a DIRECTORY mount is a
 * whole tree. `overlay_disk` is present exactly when the mode is
 * `ephemeral`.
 */
export const FuseMount = z
  .object({
    /** Which provider holds the volume. */
    provider: Identity,
    /** Which of that provider's volumes, by name. Naming it takes the `mount` grant over it. */
    volume_name: z.string(),
    /** Where in that volume, as path components from its root; empty is the volume itself. */
    volume_relative_path: z.array(z.string()),
    /** The mode the volume is meant to be in. */
    volume_mode: Mode,
    /** The most bytes the FUSE serve's own layer may hold, for an ephemeral volume: present exactly when the mode is `ephemeral`. */
    overlay_disk: z.number().int().nonnegative().optional(),
    /** Where the mount appears inside the container, as path components from the root. */
    container_path: z.array(z.string()),
  })
  .refine((mount) => (mount.overlay_disk !== undefined) === (mount.volume_mode === "ephemeral"), {
    message: "overlay_disk is present exactly when volume_mode is ephemeral",
    path: ["overlay_disk"],
  });
export type FuseMount = z.infer<typeof FuseMount>;
