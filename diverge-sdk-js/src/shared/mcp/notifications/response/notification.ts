// What a server says on its own account.

import type { z } from "zod";
import { NotificationSchema, ServerNotificationSchema } from "@modelcontextprotocol/core";

/**
 * One notification, the whole of MCP's union: narrowing it would
 * mean deciding on a consumer's behalf what is worth hearing. The
 * nine the specification names, and after them any
 * `{method, params?}` at all — what rmcp reads as a custom
 * notification — so that one of a kind this package does not know
 * arrives as a notification rather than as a failure.
 */
export const Notification = ServerNotificationSchema.or(NotificationSchema);
export type Notification = z.infer<typeof Notification>;
