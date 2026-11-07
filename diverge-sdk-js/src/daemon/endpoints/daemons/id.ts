// A remote daemon's record, by what it reaches and as whom.

import { z } from "zod";

/**
 * A remote daemon's record, by the hash of what it reaches and as whom:
 * the one way every relay, every connected tool, every key, reference
 * and attestation, and every list filter and grant names a record. A
 * base62 string — `0-9A-Za-z`, the shortest form, one to eleven
 * characters — of a 64-bit value: the first eight bytes, big-endian, of
 * the SHA-256 of the record's `daemon`, `identity`, `providers`, `ips`
 * and `addresses`, each list sorted and an absent one empty. The daemon
 * computes it and answers it on an add; this SDK carries it and never
 * computes one. Nothing else of a record is in it, so a record cannot be
 * changed to reach elsewhere under the same ID: a hop that resolves an
 * ID resolves exactly the record the caller meant, or none.
 */
export const DaemonId = z.string().regex(/^[0-9A-Za-z]{1,11}$/);
export type DaemonId = z.infer<typeof DaemonId>;
