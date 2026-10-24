// The members an agent's create and a tool's create share.

import { z } from "zod";
import { FuseMount } from "../endpoints/agents/create/client/request/fuse_mount.js";
import { Provider } from "../endpoints/agents/create/client/request/provider.js";

/**
 * Everything a container is made of that its template does not say
 * and its name is not: the template, the secrets' vault keys, the
 * provider pin and its volumes, the FUSE mounts. Flattened into an
 * agent's and a tool's create, so its members are the request's
 * own.
 */
export const Inner = z.object({
  /** The template the container is made from, by its id. */
  template: z.string(),
  /** Which vault key each secret the template declares is: the secret's name to the key. Every name the template declares, and no other. Absent, the template declares none. */
  secrets: z.record(z.string(), z.string()).optional(),
  /** The one provider the container runs on, and the volumes of that provider made visible inside it. Absent, whichever the daemon chooses, and no volume. */
  provider: Provider.optional(),
  /** Files of providers' volumes served LIVE across the daemon, mounted one each over FUSE. */
  fuse_file_mounts: z.array(FuseMount).optional(),
  /** Directories of providers' volumes served LIVE across the daemon, mounted one each over FUSE. */
  fuse_directory_mounts: z.array(FuseMount).optional(),
});
export type Inner = z.infer<typeof Inner>;
