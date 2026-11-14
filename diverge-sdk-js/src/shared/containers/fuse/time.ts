// A time, as a file records one.

import { z } from "zod";
import { concat, takeU32, takeU64, u32, u64 } from "./bytes.js";

/**
 * A time since the Unix epoch: whole seconds, and the nanoseconds past
 * them. Twelve bytes on the wire, the seconds as a `u64` and the
 * nanoseconds as a `u32`, both big-endian.
 */
export const Time = z.object({
  /** Whole seconds since the epoch. */
  secs: z.number().int().nonnegative(),
  /** Nanoseconds past the second. */
  nanos: z.number().int().nonnegative(),
});
export type Time = z.infer<typeof Time>;

/** How many bytes a time is. */
export const TIME_LEN = 12;

/** The time as its twelve bytes. */
export function encode(time: Time): Uint8Array {
  return concat(u64(time.secs), u32(time.nanos));
}

/** The time at `at`; fewer than twelve bytes there is `truncated`. */
export function decode(bytes: Uint8Array, at: number): Time {
  return { secs: takeU64(bytes, at), nanos: takeU32(bytes, at + 8) };
}
