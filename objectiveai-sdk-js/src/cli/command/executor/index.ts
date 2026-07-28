export * from "./binary";
export * from "./plugin";

import type { BinaryCommandExecutor } from "./binary";
import type { PluginCommandExecutor } from "./plugin";
import type { Client } from "../../../daemon/client";

/**
 * Any of the CLI command executors. The generated per-command execute
 * functions accept one of these and call `.execute(request)` on it.
 * The daemon [`Client`] IS the daemon-HTTP executor (regular fetch
 * mode or the viewer's Tauri proxy — one class, both modes); the
 * binary and plugin executors cover the local-subprocess and
 * in-plugin surfaces.
 */
export type CommandExecutor =
  | BinaryCommandExecutor
  | PluginCommandExecutor
  | Client;
