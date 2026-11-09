// The UI scene: the game's menus, over the world with a camera of its
// own, so they hold still while the map moves: a status line, the
// legend of the board's kinds and lines, the Arrange button, and the
// outliner.

import * as Phaser from "phaser";
import { type EdgeType, TYPES } from "../grid/edges";
import { css, NEON } from "../grid/palette";
import { NAMES } from "../grid/styles";
import type { Kind } from "../world/model";
import { KIND_ORDER } from "./kinds";
import { outliner } from "./outliner";
import { ARRANGE, KINDS, type Kinds, LINES, type Lines, STATUS, type Status } from "./status";

/** Each type's sample stroke on the legend. */
const SAMPLE: Record<EdgeType, string> = { call: "──▸", attachment: "╌╌╌", creator: "o┈┈" };

/**
 * The keys that toggle each entry, kinds first, then types of line, as
 * Phaser names them; the legend does not print them. The tenth is `0`,
 * the eleventh `-`.
 */
const KEYS = ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX", "SEVEN", "EIGHT", "NINE", "ZERO", "MINUS"];

/** The legend's left edge, its inner padding, and the gap between its columns. */
const LEFT = 12;
const PAD = 10;
const GAP = 12;

/** The fill behind every legend row and the status line. */
const PLATE = 0x05070d;
const PLATE_ALPHA = 0xe0 / 0xff;

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

/**
 * One legend row: what it toggles (nothing, for Arrange), its plate,
 * and its three columns — the sample, the name and the count.
 */
type Row = {
  toggle: { kind: Kind } | { type: EdgeType } | null;
  plate: Phaser.GameObjects.Rectangle;
  icon: Phaser.GameObjects.Text;
  name: Phaser.GameObjects.Text;
  count: Phaser.GameObjects.Text;
};

/**
 * The menus, drawn over the world.
 *
 * The rules:
 *
 * 1. The status line stands at the top-left, its words as the world
 *    says (`STATUS`): waiting for the daemon, or the lists it may not
 *    read. With nothing to say it is hidden.
 * 2. When the registry holds `KINDS` and `LINES`, the legend stands
 *    beneath it: one row per kind, in its neon, then one per type of
 *    line, each with its sample, its name and, once drawn, how many the
 *    board holds; faint while hidden. Every row is one width, and the
 *    samples, names and counts each stand in one column, at the widest
 *    of that column. Clicking a row, or pressing its key (`KEYS`, not
 *    printed), shows or hides that kind or type.
 * 3. Beneath the legend, the Arrange row, in the same columns with no
 *    count; pressing it, or the `A` key, tells the board (`ARRANGE`) to
 *    arrange every node shown by the lines shown.
 * 4. At the window's right, the outliner (`outliner`): every node, by
 *    kind.
 */
export class Ui extends Phaser.Scene {
  constructor() {
    super("ui");
  }

  create() {
    // Rule 4.
    outliner(this);
    // Rule 1.
    const status = this.add.text(LEFT, 12, "", STYLE);
    const rows: Row[] = [];
    const place = () => {
      const columns = [0, 0, 0];
      let height = 0;
      for (const row of rows) {
        [row.icon, row.name, row.count].forEach((text, i) => (columns[i] = Math.max(columns[i], text.width)));
        height = Math.max(height, row.icon.height, row.name.height, row.count.height);
      }
      const at = [LEFT + PAD, LEFT + PAD + columns[0] + GAP, LEFT + PAD + columns[0] + GAP + columns[1] + GAP];
      const width = PAD + columns[0] + GAP + columns[1] + GAP + columns[2] + PAD;
      height += 6;
      let y = status.visible ? status.y + status.height + 6 : 12;
      rows.forEach((row, i) => {
        if (i === KIND_ORDER.length || i === rows.length - 1) y += 6;
        row.plate.setPosition(LEFT, y).setSize(width, height);
        row.plate.input?.hitArea.setTo(0, 0, width, height);
        [row.icon, row.name, row.count].forEach((text, c) => text.setPosition(at[c], y + (height - text.height) / 2));
        y += height + 2;
      });
    };
    const count = (value: Status) => {
      for (const row of rows) {
        if (row.toggle === null) continue;
        const n = value.kind === "drawn" ? ("kind" in row.toggle ? value.counts[row.toggle.kind] : value.counts[row.toggle.type]) : null;
        row.count.setText(n === null ? "" : String(n));
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
    const shown = (toggle: { kind: Kind } | { type: EdgeType }) =>
      "kind" in toggle ? (this.registry.get(KINDS) as Kinds)[toggle.kind] : (this.registry.get(LINES) as Lines)[toggle.type];
    const flip = (toggle: { kind: Kind } | { type: EdgeType }) => {
      if ("kind" in toggle) {
        const kinds = this.registry.get(KINDS) as Kinds;
        this.registry.set(KINDS, { ...kinds, [toggle.kind]: !kinds[toggle.kind] } satisfies Kinds);
      } else {
        const lines = this.registry.get(LINES) as Lines;
        this.registry.set(LINES, { ...lines, [toggle.type]: !lines[toggle.type] } satisfies Lines);
      }
    };
    const paint = () => {
      for (const row of rows) {
        if (row.toggle === null) continue;
        const alpha = shown(row.toggle) ? 1 : HIDDEN;
        row.icon.setAlpha(alpha);
        row.name.setAlpha(alpha);
        row.count.setAlpha(alpha);
      }
    };
    const add = (toggle: Row["toggle"], sample: string, label: string, color: string, press: () => void, key: string) => {
      const style = { fontFamily: STYLE.fontFamily, fontSize: "12px", color };
      const plate = this.add.rectangle(0, 0, 1, 1, PLATE, PLATE_ALPHA).setOrigin(0, 0);
      plate.setInteractive({ useHandCursor: true }).on("pointerup", press);
      this.input.keyboard?.on(`keydown-${key}`, press);
      const text = (words: string) => this.add.text(0, 0, words, style);
      rows.push({ toggle, plate, icon: text(sample), name: text(label), count: text("") });
    };
    KIND_ORDER.forEach(([kind, sample, name]) => add({ kind }, sample, name, css(NEON[kind]), () => flip({ kind }), KEYS[rows.length]));
    for (const type of TYPES) add({ type }, SAMPLE[type], NAMES[type], "#22e5ff", () => flip({ type }), KEYS[rows.length]);
    // Rule 3.
    add(null, "◇", "arrange", "#ffffff", () => this.game.events.emit(ARRANGE), "A");
    tell(this.registry.get(STATUS) as Status);
    paint();
    this.registry.events.on(`changedata-${KINDS}`, paint);
    this.registry.events.on(`changedata-${LINES}`, paint);
  }
}
