// The UI scene: the game's menus, over the world with a camera of its
// own, so they hold still while the map moves. For now, a status line
// and a panel naming what is under the pointer.

import * as Phaser from "phaser";
import { ago } from "../sheet/ago";
import type { Kind, Live } from "../world/model";
import { HOVER, type Hover, STATUS, type Status } from "./status";

/** Each kind's name, one and many, in the order the status line gives them. */
const NAMES: [Kind | "links", string, string][] = [
  ["account", "account", "accounts"],
  ["agent", "agent", "agents"],
  ["standalone_tool", "standalone tool", "standalone tools"],
  ["dependency_tool", "dependency tool", "dependency tools"],
  ["links", "link", "links"],
];

/** Each kind's name on the hover panel. */
const KINDS: Record<Kind, string> = { account: "Account", agent: "Agent", standalone_tool: "Standalone tool", dependency_tool: "Dependency tool" };

/** The status line's words. */
/** An entity's live state, as its hover says it: "active now", "last connected 3 minutes ago". */
function liveWords(live: Live): string {
  const verb = live.kind === "active" ? "active" : "connected";
  if (live.on) return `${verb} now`;
  return live.at ? `last ${verb} ${ago(new Date(live.at))}` : `never ${verb}`;
}

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
    const panel = this.add.text(12, 0, "", STYLE).setOrigin(0, 1).setVisible(false);
    const place = () => panel.setY(this.scale.height - 12);
    place();
    this.scale.on("resize", place);
    this.registry.events.on(`changedata-${STATUS}`, (_parent: unknown, value: Status) => status.setText(words(value)));
    this.registry.events.on(`changedata-${HOVER}`, (_parent: unknown, value: Hover) => {
      if (value === null) {
        panel.setVisible(false);
        return;
      }
      panel.setText([KINDS[value.kind], value.detail, ...(value.live ? [liveWords(value.live)] : [])].join("\n")).setVisible(true);
    });
  }
}
