// An MCP server's error, relayed whole.

import type { z } from "zod";
import { JSONRPCErrorResponseSchema } from "@modelcontextprotocol/core";

/**
 * An MCP server's error, as JSON-RPC states one: the code, a message,
 * and data of the sender's. Carried whole rather than flattened into
 * `shared.error.Error`, since the code is CONTENT — `-32601` is "no
 * such tool", `-32602` is "the arguments were wrong" — and a relay
 * that lost it would be deciding that an MCP error means less than
 * MCP says it does. MCP's own schema, the `error` member of a
 * JSON-RPC error response.
 */
export const ErrorData = JSONRPCErrorResponseSchema.shape.error;
export type ErrorData = z.infer<typeof ErrorData>;
