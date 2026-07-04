export * from "./binary";
export * from "./plugin";
export * from "./viewerPlugin";
export * from "./websocket";

import type { BinaryCommandExecutor } from "./binary";
import type { PluginCommandExecutor } from "./plugin";
import type { ViewerPluginExecutor } from "./viewerPlugin";
import type { WebSocketExecutor } from "./websocket";

/**
 * Any of the CLI command executors. The generated per-command execute
 * functions accept one of these and call `.execute(request)` on it.
 */
export type CommandExecutor =
  | BinaryCommandExecutor
  | PluginCommandExecutor
  | ViewerPluginExecutor
  | WebSocketExecutor;
