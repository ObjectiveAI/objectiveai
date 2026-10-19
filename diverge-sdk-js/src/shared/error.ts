// The one failure.

import { z } from "zod";

/**
 * A failure, as every endpoint answers one: a JSON value and nothing
 * more. It says so little on purpose — a failure that means
 * different things in different modules is a failure every consumer
 * has to learn twice, so the value is the server's to shape and the
 * client's to show. It is JSON even inside a postcard payload, since
 * an arbitrary value has no form in a format with no
 * self-description.
 */
export const Error = z.json();
export type Error = z.infer<typeof Error>;
