// Why a run ended unexpectedly.

import { z } from "zod";

/**
 * Why a run ended without anyone asking it to, as its `undeployed`
 * says: the provider's connection ended, the provider was disabled or
 * its record edited, the daemon stopped, the container exited on its
 * own, the run's stream failed — the `error` kept before says how —
 * or a FUSE mount of the run's lost its serve: the provider that
 * holds the mounted volume went, or the connection to the daemon the
 * volume is on went, at any hop.
 */
export const Reason = z.enum(["provider_lost", "provider_disabled", "provider_edited", "daemon_stopped", "exited", "failed", "mount_lost"]);
export type Reason = z.infer<typeof Reason>;
