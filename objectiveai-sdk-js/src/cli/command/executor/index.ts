export * from "./binary";
export * from "./plugin";
export * from "./sse";
export * from "./viewer";

import type { BinaryCommandExecutor } from "./binary";
import type { PluginCommandExecutor } from "./plugin";
import type { SseCommandExecutor } from "./sse";
import type { ViewerCommandExecutor } from "./viewer";

/**
 * Any of the CLI command executors. The generated per-command execute
 * functions accept one of these and call `.execute(request)` on it.
 */
export type CommandExecutor =
  | BinaryCommandExecutor
  | PluginCommandExecutor
  | SseCommandExecutor
  | ViewerCommandExecutor;
