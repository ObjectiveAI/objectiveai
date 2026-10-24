// One container connection open through the database.

import { z } from "zod";
import type { Keyed } from "../../../shared/listing/keyed.js";
import { Container } from "./container.js";

/**
 * One connection a container holds open through the database now:
 * the daemon's number for it, the container, and when the connection
 * was opened. A container holding several is listed once per
 * connection, each by its id. A list sends them oldest opened first.
 */
export const Connection = z.object({
  /** The daemon's number for this connection, given when it is opened and never again. */
  id: z.number().int().nonnegative(),
  /** The container holding it. */
  container: Container,
  /** When the container opened it, by the daemon's clock: an RFC 3339 timestamp in UTC. */
  opened: z.iso.datetime({ offset: true }),
});
export type Connection = z.infer<typeof Connection>;

/** Keyed by the id. */
export const key: Keyed<Connection> = (connection) => connection.id;
