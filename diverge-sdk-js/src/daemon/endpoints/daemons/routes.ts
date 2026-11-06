// Which routes a remote daemon's record may be reached by.

import { z } from "zod";
import { Ip } from "../providers/ip.js";
import { Identity } from "../../../shared/identity.js";

/**
 * The routes a record lets the daemon reach its remote by: three
 * filters, each optional, beside the record's own members. Absent is
 * no filter; within a filter any entry matches; across filters all must
 * match; an empty list matches nothing; `addresses` matches only a
 * route the provider dialled. Nothing is checked at add or edit.
 */
export const Routes = z.object({
  /** The providers it may be reached through. Absent, any. */
  providers: z.array(Identity).optional(),
  /** The IPs, or blocks, its connection to that provider may be at. Absent, any. */
  ips: z.array(Ip).optional(),
  /** The dial targets that provider may have dialled it at. Absent, any. */
  addresses: z.array(z.string()).optional(),
});
export type Routes = z.infer<typeof Routes>;
