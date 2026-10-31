// The UI scene: the game's menus, over the world with a camera of its
// own, so they hold still while the map moves. For now, a status line.

import * as Phaser from "phaser";
import type { Kind } from "../world/model";
import { STATUS, type Status } from "./status";

/** Each kind's name, one and many, in the order the status line gives them. */
const NAMES: [Kind | "links", string, string][] = [
  ["account", "account", "accounts"],
  ["agent", "agent", "agents"],
  ["standalone_tool", "standalone tool", "standalone tools"],
  ["dependency_tool", "dependency tool", "dependency tools"],
  ["links", "link", "links"],
];

/** The status line's words. */
function words(status: Status): string {
  if (status.kind === "waiting") return status.reason === null ? "Waiting for the daemon…" : `Waiting for the daemon… (${status.reason})`;
  const line = NAMES.map(([k, one, many]) => `${status.counts[k]} ${status.counts[k] === 1 ? one : many}`).join("  ·  ");
  return status.forbidden.length === 0 ? line : `${line}\nNot allowed to list: ${status.forbidden.join(", ")}`;
}

const STYLE: Phaser.Types.GameObjects.Text.TextStyle = {
  fontFamily: "Georgia, serif",
  fontSize: "14px",
  color: "#f6eedb",
  backgroundColor: "#1a1510e0",
  padding: { x: 10, y: 6 },
};

/** The menus, drawn over the world. */
export class Ui extends Phaser.Scene {
  constructor() {
    super("ui");
  }

  create() {
    const status = this.add.text(12, 12, words(this.registry.get(STATUS) as Status), STYLE);
    this.registry.events.on(`changedata-${STATUS}`, (_parent: unknown, value: Status) => status.setText(words(value)));
  }
}
