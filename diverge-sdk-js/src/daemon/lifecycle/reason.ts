// Why a run ended unexpectedly.

import { z } from "zod";

/**
 * Why a run ended without anyone asking it to, as its `undeployed`
 * says: the provider's connection ended, the provider was disabled or
 * its record edited, the daemon stopped, the container exited on its
 * own, or the run's stream failed — the `error` kept before says how.
 */
export const Reason = z.enum(["provider_lost", "provider_disabled", "provider_edited", "daemon_stopped", "exited", "failed"]);
export type Reason = z.infer<typeof Reason>;
