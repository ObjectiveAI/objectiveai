// The outliner: the game's one menu, down the right of the window, as a
// grand strategy game lists what its player holds — Arrange and the
// types of line at its top, a row each, what the world has to say
// beneath, and every node on the board by kind, each kind shown or
// hidden by its eye.

import * as Phaser from "phaser";
import { type EdgeType, TYPES } from "../grid/edges";
import { css, NEON } from "../grid/palette";
import type { Kind } from "../world/model";
import { KIND_ORDER } from "./kinds";
import { ARRANGE, type Bounds, FOCUS, HOVER, KINDS, type Kinds, LINES, type Lines, OUTLINE, OUTLINER, type Outline, SELECTED, STATUS, type Status } from "./status";

/** The outliner's width, its gap from the window's edges, and its inner padding. */
const WIDTH = 290;
const EDGE = 12;
const PAD = 8;

/** One row's height, in pixels. */
const ROW = 20;

/** The gap under the title bar and under the status row. */
const GAP = 4;

/** The most characters a name shows; a longer one ends in an ellipsis. */
const NAME = 28;

/** How many rows one notch of the wheel scrolls. */
const NOTCH = 3;

/** The panel's fill, and a row's when hovered or selected. */
const PLATE = 0x05070d;
const PLATE_ALPHA = 0xe0 / 0xff;
const HOVERED = 0.08;
const CHOSEN = 0.16;

/** How faint a hidden kind or type of line is drawn. */
const HIDDEN = 0.35;

/** The dot of a bright entity, and of a faint one. */
const ON = "#39ff88";
const OFF = "#3a4656";

/** A section's eye: open while its kind is shown, shut while hidden. */
const EYE_OPEN = "◉";
const EYE_SHUT = "◌";

/** The types of line's colour, and the status row's. */
const CYAN = "#22e5ff";
const AMBER = "#ffb020";

const FONT = "Consolas, 'Courier New', monospace";

/** Each type's sample stroke, and its name. */
const SAMPLE: Record<EdgeType, string> = { call: "──▸", attachment: "╌╌╌", creator: "o┈┈" };
const NAMES: Record<EdgeType, string> = { call: "calls", attachment: "attachments", creator: "creators" };

/** Where a row's columns begin, from the panel's inner edge: the fold, the sample, and the name — a stroke's name further, past its three characters. */
const FOLD = 0;
const ICON = 14;
const TITLE = 30;
const STROKE_TITLE = 40;

/** One fixed row at the top: Arrange, or one type of line. */
type Fixed = { plate: Phaser.GameObjects.Rectangle; icon: Phaser.GameObjects.Text; name: Phaser.GameObjects.Text; aux: Phaser.GameObjects.Text; eye: Phaser.GameObjects.Text; type: EdgeType | null };

/** One line of the outliner: a kind's header, or one entity under it. */
type Line = { header: Kind; count: number } | { entry: Outline[number] };

/** One drawn row, reused for whichever line scrolls into it: its plate, its four columns, and a header's eye. */
type Slot = { plate: Phaser.GameObjects.Rectangle; fold: Phaser.GameObjects.Text; icon: Phaser.GameObjects.Text; name: Phaser.GameObjects.Text; aux: Phaser.GameObjects.Text; eye: Phaser.GameObjects.Text };

/** A name as a row shows it: cut at `NAME` characters, an ellipsis ending a longer one. */
const shortened = (label: string) => (label.length > NAME ? `${label.slice(0, NAME - 1)}…` : label);

/** The status row's words, or `null` when it has nothing to say. */
function words(status: Status | undefined): string | null {
  if (status === undefined) return null;
  if (status.kind === "waiting") return status.reason === null ? "Waiting for the daemon…" : `Waiting for the daemon… (${status.reason})`;
  return status.forbidden.length === 0 ? null : `Not allowed to list: ${status.forbidden.join(", ")}`;
}

/** Show a hidden kind, or hide a shown one. */
export function flipKind(registry: Phaser.Data.DataManager, kind: Kind): void {
  const kinds = registry.get(KINDS) as Kinds;
  registry.set(KINDS, { ...kinds, [kind]: !kinds[kind] } satisfies Kinds);
}

/** Show a hidden type of line, or hide a shown one. */
export function flipLine(registry: Phaser.Data.DataManager, type: EdgeType): void {
  const lines = registry.get(LINES) as Lines;
  registry.set(LINES, { ...lines, [type]: !lines[type] } satisfies Lines);
}

/**
 * Draw the outliner in the UI scene.
 *
 * The rules:
 *
 * 1. It stands at the window's top-right, `WIDTH` wide, as tall as its
 *    rows need and never taller than the window. Its bounds are told
 *    to the board (`OUTLINER`), which neither pans nor zooms for a
 *    press or a wheel that begins over it.
 * 2. Its top rows, one beneath another: Arrange — pressing it tells
 *    the board (`ARRANGE`) — then one row per type of line, its sample,
 *    its name, how many the board holds once drawn, and its eye;
 *    pressing the row shows or hides that type (`LINES`), and a hidden
 *    one's eye is shut and its row faint. A rule closes them.
 * 3. Beneath the rule, the status row says what the world says
 *    (`STATUS`) — waiting for the daemon, or the lists the account may
 *    not read — in amber, wrapped to the width; with nothing to say it
 *    is not there.
 * 4. Beneath, one section per kind, in `KIND_ORDER`'s
 *    order, from what the board tells it (`OUTLINE`): a header
 *    — `▾` or `▸`, its sample, its name, how many it holds, and its
 *    eye — then its entities by name, each with its sample and a dot,
 *    green while bright.
 * 5. Pressing a header's eye shows or hides its kind on the board
 *    (`KINDS`); a hidden kind's eye is shut and its section faint.
 *    Pressing the rest of a header folds or unfolds its section.
 * 6. Pressing an entity tells the board (`FOCUS`), which brings it to
 *    the middle of the view and opens its sheet. Hovering one tells the
 *    board (`HOVER`), which focuses its lines as hovering the node does.
 *    The row of the entity whose sheet is open (`SELECTED`) is lit.
 * 7. When the sections outgrow the window, the wheel over the outliner
 *    scrolls them `NOTCH` rows a notch; only the rows in view are drawn,
 *    and the top rows and the status row hold still.
 */
export function outliner(scene: Phaser.Scene): void {
  const folded = new Set<Kind>();
  const slots: Slot[] = [];
  let lines: Line[] = [];
  let top = 0;
  let hovered: number | null = null;
  const style = (color: string) => ({ fontFamily: FONT, fontSize: "12px", color });
  // Rule 1: the panel takes every press made on it, so the board beneath gets none.
  const panel = scene.add.rectangle(0, 0, WIDTH, 1, PLATE, PLATE_ALPHA).setOrigin(0, 0).setInteractive();
  // Rule 2.
  const fixed = (type: EdgeType | null, press: () => void): Fixed => {
    const plate = scene.add.rectangle(0, 0, WIDTH - 2, ROW, 0xffffff, 0).setOrigin(0, 0).setInteractive({ useHandCursor: true });
    plate.on("pointerup", press);
    plate.on("pointerover", () => plate.setFillStyle(0xffffff, HOVERED));
    plate.on("pointerout", () => plate.setFillStyle(0xffffff, 0));
    const color = type === null ? "#ffffff" : CYAN;
    const text = (words: string) => scene.add.text(0, 0, words, style(color));
    return { plate, icon: text(type === null ? "◇" : SAMPLE[type]), name: text(type === null ? "arrange" : NAMES[type]), aux: text(""), eye: text(""), type };
  };
  const tops = [fixed(null, () => scene.game.events.emit(ARRANGE)), ...TYPES.map((type) => fixed(type, () => flipLine(scene.registry, type)))];
  const rule = scene.add.rectangle(0, 0, WIDTH - 2 * PAD, 1, 0x22e5ff, 0.25).setOrigin(0, 0);
  // Rule 3.
  const status = scene.add.text(0, 0, "", { ...style(AMBER), wordWrap: { width: WIDTH - 2 * PAD } });
  const slot = (): Slot => {
    const plate = scene.add.rectangle(0, 0, WIDTH - 2, ROW, 0xffffff, 0).setOrigin(0, 0);
    const text = () => scene.add.text(0, 0, "", style("#d8e4f0"));
    const index = slots.length;
    const made: Slot = { plate, fold: text(), icon: text(), name: text(), aux: text(), eye: text() };
    plate.setInteractive({ useHandCursor: true });
    plate.on("pointerup", () => press(top + index));
    plate.on("pointerover", () => enter(top + index));
    plate.on("pointerout", () => leave(top + index));
    made.eye.setInteractive({ useHandCursor: true });
    made.eye.on("pointerup", () => peek(top + index));
    return made;
  };
  // Rule 4.
  const gather = () => {
    const outline = (scene.registry.get(OUTLINE) as Outline | undefined) ?? [];
    lines = [];
    for (const [kind] of KIND_ORDER) {
      const entries = outline.filter((entry) => entry.kind === kind).sort((a, b) => a.label.localeCompare(b.label));
      lines.push({ header: kind, count: entries.length });
      if (!folded.has(kind)) for (const entry of entries) lines.push({ entry });
    }
  };
  const draw = () => {
    const kinds = scene.registry.get(KINDS) as Kinds | undefined;
    const shownLines = scene.registry.get(LINES) as Lines | undefined;
    const value = scene.registry.get(STATUS) as Status | undefined;
    const selected = scene.registry.get(SELECTED) as string | null | undefined;
    const x = scene.scale.width - EDGE - WIDTH;
    // Rule 2: Arrange, then each type of line, a row each.
    let y = EDGE + PAD;
    for (const row of tops) {
      row.plate.setPosition(x + 1, y);
      row.icon.setPosition(x + PAD + ICON, y + 3);
      row.name.setPosition(x + PAD + (row.type === null ? TITLE : STROKE_TITLE), y + 3);
      if (row.type !== null) {
        const on = shownLines === undefined || shownLines[row.type];
        row.eye.setText(on ? EYE_OPEN : EYE_SHUT).setPosition(x + WIDTH - PAD - row.eye.width, y + 3);
        row.aux.setText(value?.kind === "drawn" ? String(value.counts[row.type]) : "").setPosition(row.eye.x - 10 - row.aux.width, y + 3);
        for (const part of [row.icon, row.name, row.aux, row.eye]) part.setAlpha(on ? 1 : HIDDEN);
      }
      y += ROW;
    }
    rule.setPosition(x + PAD, y + GAP / 2);
    y += GAP;
    // Rule 3.
    const said = words(value);
    status.setText(said ?? "").setVisible(said !== null).setPosition(x + PAD, y + 2);
    if (said !== null) y += status.height + 4 + GAP;
    // Rule 7: as many rows as the window leaves room for.
    const fit = Math.max(1, Math.floor((scene.scale.height - EDGE - PAD - y) / ROW));
    const shown = Math.min(lines.length, fit);
    top = Math.max(0, Math.min(top, lines.length - shown));
    while (slots.length < shown) slots.push(slot());
    const height = y - EDGE + shown * ROW + PAD;
    panel.setPosition(x, EDGE).setSize(WIDTH, height);
    panel.input?.hitArea.setTo(0, 0, WIDTH, height);
    scene.registry.set(OUTLINER, { x, y: EDGE, width: WIDTH, height } satisfies Bounds);
    slots.forEach((s, i) => {
      const line = lines[top + i];
      const visible = i < shown && line !== undefined;
      for (const part of [s.plate, s.fold, s.icon, s.name, s.aux, s.eye]) part.setVisible(visible);
      if (!visible) return;
      const at = y + i * ROW;
      s.plate.setPosition(x + 1, at);
      const kind = "header" in line ? line.header : line.entry.kind;
      const [, sample, title] = KIND_ORDER.find(([k]) => k === kind) ?? [kind, "", kind];
      const color = css(NEON[kind]);
      const alpha = kinds === undefined || kinds[kind] ? 1 : HIDDEN;
      if ("header" in line) {
        // Rule 5.
        s.eye.setText(alpha === 1 ? EYE_OPEN : EYE_SHUT).setColor(color).setVisible(true);
        s.eye.setPosition(x + WIDTH - PAD - s.eye.width, at + 3);
        s.fold.setText(folded.has(kind) ? "▸" : "▾").setColor(color).setPosition(x + PAD + FOLD, at + 3);
        s.icon.setText(sample).setColor(color).setPosition(x + PAD + ICON, at + 3);
        s.name.setText(title).setColor(color).setFontStyle("bold").setPosition(x + PAD + TITLE, at + 3);
        s.aux.setText(String(line.count)).setColor(color).setPosition(s.eye.x - 10 - s.aux.width, at + 3);
        s.plate.setFillStyle(0xffffff, top + i === hovered ? HOVERED : 0);
      } else {
        s.eye.setVisible(false);
        s.fold.setText("");
        s.icon.setText(sample).setColor(color).setPosition(x + PAD + 30, at + 3);
        s.name.setText(shortened(line.entry.label)).setColor("#d8e4f0").setFontStyle("").setPosition(x + PAD + 46, at + 3);
        s.aux.setText("●").setColor(line.entry.bright ? ON : OFF).setPosition(x + WIDTH - PAD - s.aux.width, at + 3);
        s.plate.setFillStyle(0xffffff, line.entry.key === selected ? CHOSEN : top + i === hovered ? HOVERED : 0);
      }
      for (const part of [s.fold, s.icon, s.name, s.aux]) part.setAlpha(alpha);
    });
  };
  const redraw = () => {
    gather();
    draw();
  };
  // Rules 5 and 6.
  const press = (at: number) => {
    const line = lines[at];
    if (line === undefined) return;
    if ("header" in line) {
      if (folded.has(line.header)) folded.delete(line.header);
      else folded.add(line.header);
      redraw();
    } else scene.game.events.emit(FOCUS, line.entry.key);
  };
  const peek = (at: number) => {
    const line = lines[at];
    if (line !== undefined && "header" in line) flipKind(scene.registry, line.header);
  };
  const enter = (at: number) => {
    hovered = at;
    const line = lines[at];
    if (line !== undefined && "entry" in line) scene.game.events.emit(HOVER, line.entry.key);
    draw();
  };
  const leave = (at: number) => {
    if (hovered !== at) return;
    hovered = null;
    scene.game.events.emit(HOVER, null);
    draw();
  };
  // Rule 7.
  scene.input.on("wheel", (pointer: Phaser.Input.Pointer, _over: unknown, _dx: number, dy: number) => {
    const bounds = scene.registry.get(OUTLINER) as Bounds | undefined;
    if (bounds === undefined || pointer.x < bounds.x || pointer.x > bounds.x + bounds.width || pointer.y < bounds.y || pointer.y > bounds.y + bounds.height) return;
    top += dy > 0 ? NOTCH : -NOTCH;
    if (hovered !== null) {
      hovered = null;
      scene.game.events.emit(HOVER, null);
    }
    draw();
  });
  const watched = [OUTLINE, SELECTED, KINDS, LINES, STATUS];
  const on = (key: string) => (key === OUTLINE ? redraw : draw);
  for (const key of watched) scene.registry.events.on(`changedata-${key}`, on(key));
  scene.scale.on("resize", draw);
  scene.events.once("shutdown", () => {
    for (const key of watched) scene.registry.events.off(`changedata-${key}`, on(key));
    scene.scale.off("resize", draw);
  });
  redraw();
}
