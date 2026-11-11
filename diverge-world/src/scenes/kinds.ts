// The kinds as the menus show them: in order, each with its sample
// shape and its name.

import type { Kind } from "../world/model";

/** The kinds, in the outliner's order, each with its sample shape and its name. */
export const KIND_ORDER: [Kind, string, string][] = [
  ["account", "◎", "accounts"],
  ["agent", "⬡", "agents"],
  ["standalone_tool", "◇", "standalone tools"],
  ["connected_standalone_tool", "◈", "connected tools"],
  ["daemon", "⊕", "daemons"],
  ["dependency_tool", "□", "dependency tools"],
  ["volume", "⛁", "volumes"],
  ["provider", "▤", "providers"],
];
