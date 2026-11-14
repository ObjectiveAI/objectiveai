// A file or a directory of any provider's volume, served live.

import { z } from "zod";
import { Mode } from "../../../../../../provider/endpoints/volumes/mode.js";
import { Identity } from "../../../../../agent_log/identity.js";
import { DaemonId } from "../../../../daemons/id.js";

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
 *
 * A mount may name a volume on another daemon: `daemon` is this
 * daemon's record of it, and `route` the records beyond it. The
 * provider and the volume are then that daemon's, named as it names
 * them. Naming one takes the daemons grant's `relay` over the record
 * and nothing else here; at the run's start the daemon opens that
 * daemon's `volumes.fuse_serve` of the volume, as the account its
 * record names, and whether that account is served it is that
 * daemon's to judge. A serve refused is the start's error. The record
 * is not deleted while the agent or the tool that names it exists.
 *
 * A serve that ends before the run does — the volume's provider
 * gone, or the connection to the volume's daemon gone, at any hop —
 * ends the run as failed, its reason `mount_lost`.
 */
export const FuseMount = z
  .object({
    /** The remote daemon the volume is on, by the ID of this daemon's record of it. Absent, the volume is a provider's of this daemon's own. An ID no record has is the create's error; the caller must hold the daemons grant's `relay` over the record. */
    daemon: DaemonId.optional(),
    /** The remote-daemon records the serve is relayed through beyond `daemon`, each by its ID on the daemon before it: the volume is on the last. Absent when empty, and then the volume is on `daemon`. Stated only with `daemon`: a route with no daemon is the create's error. */
    route: z.array(DaemonId).optional(),
    /** Which provider holds the volume. For a volume on another daemon, a provider of THAT daemon's, which this daemon does not check. */
    provider: Identity,
    /** Which of that provider's volumes, by name. Naming it takes the `mount` grant over it; for a volume on another daemon, `relay` over the daemon's record is the grant, and the volume is that daemon's to find. */
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
