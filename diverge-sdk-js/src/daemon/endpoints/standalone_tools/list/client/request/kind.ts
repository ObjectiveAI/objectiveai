// Created, or connected.

import { z } from "zod";

/**
 * Created by the daemon from a template, or connected to a container
 * somebody else runs: the `kind` the tool's origin is tagged with,
 * as a value of its own to narrow a list by. Snake case on the wire:
 * `"created"`, `"connected"`.
 */
export const Kind = z.enum(["created", "connected"]);
export type Kind = z.infer<typeof Kind>;
