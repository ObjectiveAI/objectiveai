// The UI scene: the game's menus, over the world with a camera of its
// own, so they hold still while the map moves. For now, a status line,
// the legend of the board's kinds and lines, and the Arrange button.

import * as Phaser from "phaser";
import { type EdgeType, TYPES } from "../grid/edges";
import { css, NEON } from "../grid/palette";
import { NAMES } from "../grid/styles";
import type { Kind } from "../world/model";
import { ARRANGE, KINDS, type Kinds, LINES, type Lines, STATUS, type Status } from "./status";

/** The kinds, in the legend's order, each with its sample shape and its name. */
const KIND_ORDER: [Kind, string, string][] = [
  ["account", "◎", "accounts"],
  ["agent", "⬡", "agents"],
  ["standalone_tool", "◇", "standalone tools"],
  ["connected_standalone_tool", "◈", "connected tools"],
  ["daemon", "⊕", "remote daemons"],
  ["dependency_tool", "□", "dependency tools"],
  ["volume", "⛁", "volumes"],
  ["provider", "▤", "providers"],
];

/** Each type's sample stroke on the legend. */
const SAMPLE: Record<EdgeType, string> = { call: "──▸", attachment: "╌╌╌", creator: "o┈┈" };

/**
 * The keys that toggle each entry, kinds first, then types of line:
 * each as Phaser names it, and as the legend prints it. The tenth is
 * `0`, the eleventh `-`.
 */
const KEYS: [string, string][] = [
  ["ONE", "1"],
  ["TWO", "2"],
  ["THREE", "3"],
  ["FOUR", "4"],
  ["FIVE", "5"],
  ["SIX", "6"],
  ["SEVEN", "7"],
  ["EIGHT", "8"],
  ["NINE", "9"],
  ["ZERO", "0"],
  ["MINUS", "-"],
];

/** The width a name is padded to, so the counts stand in one column. */
const NAME_WIDTH = 18;

/** How faint a hidden entry is drawn. */
const HIDDEN = 0.35;

/** The status line's words, or `null` when it has nothing to say. */
function words(status: Status): string | null {
  if (status.kind === "waiting") return status.reason === null ? "Waiting for the daemon…" : `Waiting for the daemon… (${status.reason})`;
  return status.forbidden.length === 0 ? null : `Not allowed to list: ${status.forbidden.join(", ")}`;
}

const STYLE: Phaser.Types.GameObjects.Text.TextStyle = {
  fontFamily: "Consolas, 'Courier New', monospace",
  fontSize: "13px",
  color: "#22e5ff",
  backgroundColor: "#05070de0",
  padding: { x: 10, y: 6 },
};

/** One legend entry: what it toggles, its key and sample, its name, and its text. */
type Entry = { toggle: { kind: Kind } | { type: EdgeType }; head: string; name: string; text: Phaser.GameObjects.Text };

/** An entry's words: its key and sample, its name, and its count once there is one. */
const say = (entry: Entry, n: number | null) => (n === null ? `${entry.head}  ${entry.name}` : `${entry.head}  ${entry.name.padEnd(NAME_WIDTH)}${n}`);

/**
 * The menus, drawn over the world.
 *
 * The rules:
 *
 * 1. The status line stands at the top-left, its words as the world
 *    says (`STATUS`): waiting for the daemon, or the lists it may not
 *    read. With nothing to say it is hidden.
 * 2. When the registry holds `KINDS` and `LINES`, the legend stands
 *    beneath it: one entry per kind, in its neon, then one per type of
 *    line, each with its key, its sample, its name and, once drawn, how
 *    many the board holds; faint while hidden. Clicking an entry, or
 *    pressing its key, shows or hides that kind or type.
 * 3. Beneath the legend, the Arrange button; pressing it, or the `A`
 *    key, tells the board (`ARRANGE`) to arrange every node shown by
 *    the lines shown.
 */
export class Ui extends Phaser.Scene {
  constructor() {
    super("ui");
  }

  create() {
    // Rule 1.
    const status = this.add.text(12, 12, "", STYLE);
    const entries: Entry[] = [];
    let button: Phaser.GameObjects.Text | null = null;
    const place = () => {
      let y = status.visible ? status.y + status.height + 6 : 12;
      entries.forEach((entry, i) => {
        if (i === KIND_ORDER.length) y += 6;
        entry.text.setPosition(12, y);
        y += entry.text.height + 2;
      });
      button?.setPosition(12, y + 6);
    };
    const count = (value: Status) => {
      for (const entry of entries) {
        const n = value.kind === "drawn" ? ("kind" in entry.toggle ? value.counts[entry.toggle.kind] : value.counts[entry.toggle.type]) : null;
        entry.text.setText(say(entry, n));
      }
    };
    const tell = (value: Status) => {
      const said = words(value);
      status.setText(said ?? "").setVisible(said !== null);
      count(value);
      place();
    };
    this.registry.events.on(`changedata-${STATUS}`, (_parent: unknown, value: Status) => tell(value));
    // Rule 2.
    if (this.registry.get(KINDS) === undefined || this.registry.get(LINES) === undefined) {
      tell(this.registry.get(STATUS) as Status);
      return;
    }
    const toggle = (entry: Entry) => {
      if ("kind" in entry.toggle) {
        const kinds = this.registry.get(KINDS) as Kinds;
        this.registry.set(KINDS, { ...kinds, [entry.toggle.kind]: !kinds[entry.toggle.kind] } satisfies Kinds);
      } else {
        const lines = this.registry.get(LINES) as Lines;
        this.registry.set(LINES, { ...lines, [entry.toggle.type]: !lines[entry.toggle.type] } satisfies Lines);
      }
    };
    const paint = () => {
      const kinds = this.registry.get(KINDS) as Kinds;
      const lines = this.registry.get(LINES) as Lines;
      for (const entry of entries) entry.text.setAlpha(("kind" in entry.toggle ? kinds[entry.toggle.kind] : lines[entry.toggle.type]) ? 1 : HIDDEN);
    };
    const add = (what: Entry["toggle"], sample: string, name: string, color: string) => {
      const text = this.add.text(0, 0, "", { ...STYLE, fontSize: "12px", color, padding: { x: 10, y: 3 } });
      const [code, label] = KEYS[entries.length];
      const entry = { toggle: what, head: `${label}  ${sample}`, name, text };
      text.setInteractive({ useHandCursor: true }).on("pointerup", () => toggle(entry));
      this.input.keyboard?.on(`keydown-${code}`, () => toggle(entry));
      entries.push(entry);
    };
    for (const [kind, sample, name] of KIND_ORDER) add({ kind }, sample, name, css(NEON[kind]));
    for (const type of TYPES) add({ type }, SAMPLE[type], NAMES[type], "#22e5ff");
    // Rule 3.
    const arrange = () => this.game.events.emit(ARRANGE);
    button = this.add.text(0, 0, "A  ◇ arrange", { ...STYLE, fontSize: "12px", color: "#ffffff", padding: { x: 10, y: 4 } });
    button.setInteractive({ useHandCursor: true }).on("pointerup", arrange);
    this.input.keyboard?.on("keydown-A", arrange);
    tell(this.registry.get(STATUS) as Status);
    paint();
    this.registry.events.on(`changedata-${KINDS}`, paint);
    this.registry.events.on(`changedata-${LINES}`, paint);
  }
}
