// The agent as a patch sees it.

import { z } from "zod";
import { FuseMount } from "../../../create/client/request/fuse_mount.js";
import { VolumeMount } from "../../../create/client/request/volume_mount.js";

/**
 * The members of an agent an edit may change, as one JSON document
 * — and what its result must be again: every member present, a
 * member the patch takes away read as absent or empty, and no
 * member this type does not have. Not here: the template, the
 * index, the provider, the creator, the created time. On an agent
 * pinned to no provider `volume_mounts` stays empty, and giving it
 * one is the edit's error. `secrets` names every secret the
 * template declares and no other; a key named anew takes the `use`
 * grant and holds at least one value.
 */
export const Editable = z.strictObject({
  /** The name, or `null` for none. */
  name: z.string().nullable(),
  /** The pinned provider's volumes mounted into the container, in order. */
  volume_mounts: z.array(VolumeMount),
  /** Files of providers' volumes served live over FUSE. */
  fuse_file_mounts: z.array(FuseMount),
  /** Directories of providers' volumes served live over FUSE. */
  fuse_directory_mounts: z.array(FuseMount),
  /** Which vault key each secret the template declares is: the secret's name to the key. */
  secrets: z.record(z.string(), z.string()),
});
export type Editable = z.infer<typeof Editable>;
