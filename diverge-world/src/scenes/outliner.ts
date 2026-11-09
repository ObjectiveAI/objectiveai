// The outliner: every node on the board, listed by kind down the right
// of the window, as a grand strategy game lists what its player holds.

import * as Phaser from "phaser";
import { css, NEON } from "../grid/palette";
import type { Kind } from "../world/model";
import { KIND_ORDER } from "./kinds";
import { type Bounds, FOCUS, HOVER, KINDS, type Kinds, OUTLINE, OUTLINER, type Outline, SELECTED } from "./status";

/** The outliner's width, its gap from the window's edges, and its inner padding. */
const WIDTH = 270;
const EDGE = 12;
const PAD = 8;

/** One row's height, in pixels. */
const ROW = 20;

/** The most characters a name shows; a longer one ends in an ellipsis. */
const NAME = 28;

/** How many rows one notch of the wheel scrolls. */
const NOTCH = 3;

/** The panel's fill, and a row's when hovered or selected. */
const PLATE = 0x05070d;
const PLATE_ALPHA = 0xe0 / 0xff;
const HOVERED = 0.08;
const CHOSEN = 0.16;

/** How faint a hidden kind's section is drawn. */
const HIDDEN = 0.35;

/** The dot of a bright entity, and of a faint one. */
const ON = "#39ff88";
const OFF = "#3a4656";

const FONT = "Consolas, 'Courier New', monospace";

/** One line of the outliner: a kind's header, or one entity under it. */
type Line = { header: Kind; count: number } | { entry: Outline[number] };

/** One drawn row, reused for whichever line scrolls into it. */
type Slot = { plate: Phaser.GameObjects.Rectangle; icon: Phaser.GameObjects.Text; name: Phaser.GameObjects.Text; aux: Phaser.GameObjects.Text };

/** A name as a row shows it: cut at `NAME` characters, an ellipsis ending a longer one. */
const shortened = (label: string) => (label.length > NAME ? `${label.slice(0, NAME - 1)}…` : label);

/**
 * Draw the outliner in the UI scene.
 *
 * The rules:
 *
 * 1. It stands at the window's top-right, `WIDTH` wide, as tall as its
 *    lines need and never taller than the window. Its bounds are told
 *    to the board (`OUTLINER`), which neither pans nor zooms for a
 *    press or a wheel that begins over it.
 * 2. It lists what the board tells it (`OUTLINE`): one section per
 *    kind, in the legend's order (`KIND_ORDER`), each a header — its
 *    sample, its name and how many it holds — then its entities by
 *    name, each with its sample and a dot, green while bright. A
 *    section whose kind the legend hides is drawn faint.
 * 3. Clicking a header folds or unfolds its section; a folded header
 *    shows `▸`, an unfolded one `▾`.
 * 4. Clicking an entity tells the board (`FOCUS`), which brings it to
 *    the middle of the view and opens its sheet. Hovering one tells the
 *    board (`HOVER`), which focuses its lines as hovering the node does.
 *    The row of the entity whose sheet is open (`SELECTED`) is lit.
 * 5. When the lines outgrow the window, the wheel over the outliner
 *    scrolls it `NOTCH` rows a notch; only the rows in view are drawn.
 */
export function outliner(scene: Phaser.Scene): void {
  const folded = new Set<Kind>();
  const slots: Slot[] = [];
  let lines: Line[] = [];
  let top = 0;
  let hovered: number | null = null;
  // Rule 1: the panel takes every press made on it, so the board beneath gets none.
  const panel = scene.add.rectangle(0, 0, WIDTH, 1, PLATE, PLATE_ALPHA).setOrigin(0, 0).setInteractive();
  const fit = () => Math.max(1, Math.floor((scene.scale.height - 2 * EDGE - 2 * PAD) / ROW));
  const slot = (): Slot => {
    const plate = scene.add.rectangle(0, 0, WIDTH - 2, ROW, 0xffffff, 0).setOrigin(0, 0);
    const text = () => scene.add.text(0, 0, "", { fontFamily: FONT, fontSize: "12px" });
    const made: Slot = { plate, icon: text(), name: text(), aux: text() };
    const index = slots.length;
    plate.setInteractive({ useHandCursor: true });
    plate.on("pointerup", () => press(top + index));
    plate.on("pointerover", () => enter(top + index));
    plate.on("pointerout", () => leave(top + index));
    return made;
  };
  // Rule 2.
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
    const selected = scene.registry.get(SELECTED) as string | null | undefined;
    const shown = Math.min(lines.length, fit());
    top = Math.max(0, Math.min(top, lines.length - shown));
    while (slots.length < shown) slots.push(slot());
    const x = scene.scale.width - EDGE - WIDTH;
    const height = shown * ROW + 2 * PAD;
    panel.setPosition(x, EDGE).setSize(WIDTH, height);
    panel.input?.hitArea.setTo(0, 0, WIDTH, height);
    scene.registry.set(OUTLINER, { x, y: EDGE, width: WIDTH, height } satisfies Bounds);
    slots.forEach((s, i) => {
      const line = lines[top + i];
      const visible = i < shown && line !== undefined;
      for (const part of [s.plate, s.icon, s.name, s.aux]) part.setVisible(visible);
      if (!visible) return;
      const y = EDGE + PAD + i * ROW;
      s.plate.setPosition(x + 1, y);
      const kind = "header" in line ? line.header : line.entry.kind;
      const sample = KIND_ORDER.find(([k]) => k === kind)?.[1] ?? "";
      const color = css(NEON[kind]);
      const alpha = kinds === undefined || kinds[kind] ? 1 : HIDDEN;
      if ("header" in line) {
        const name = KIND_ORDER.find(([k]) => k === kind)?.[2] ?? kind;
        s.icon.setText(folded.has(kind) ? "▸" : "▾").setColor(color).setPosition(x + PAD, y + 3);
        s.name.setText(`${sample} ${name}`).setColor(color).setFontStyle("bold").setPosition(x + PAD + 16, y + 3);
        s.aux.setText(String(line.count)).setColor(color).setPosition(x + WIDTH - PAD - s.aux.width, y + 3);
        s.plate.setFillStyle(0xffffff, top + i === hovered ? HOVERED : 0);
      } else {
        s.icon.setText(sample).setColor(color).setPosition(x + PAD + 16, y + 3);
        s.name.setText(shortened(line.entry.label)).setColor("#d8e4f0").setFontStyle("").setPosition(x + PAD + 34, y + 3);
        s.aux.setText("●").setColor(line.entry.bright ? ON : OFF).setPosition(x + WIDTH - PAD - s.aux.width, y + 3);
        s.plate.setFillStyle(0xffffff, line.entry.key === selected ? CHOSEN : top + i === hovered ? HOVERED : 0);
      }
      for (const part of [s.icon, s.name, s.aux]) part.setAlpha(alpha);
    });
  };
  const redraw = () => {
    gather();
    draw();
  };
  // Rules 3 and 4.
  const press = (at: number) => {
    const line = lines[at];
    if (line === undefined) return;
    if ("header" in line) {
      if (folded.has(line.header)) folded.delete(line.header);
      else folded.add(line.header);
      redraw();
    } else scene.game.events.emit(FOCUS, line.entry.key);
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
  // Rule 5.
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
  scene.registry.events.on(`changedata-${OUTLINE}`, redraw);
  scene.registry.events.on(`changedata-${SELECTED}`, draw);
  scene.registry.events.on(`changedata-${KINDS}`, draw);
  scene.scale.on("resize", draw);
  scene.events.once("shutdown", () => {
    scene.registry.events.off(`changedata-${OUTLINE}`, redraw);
    scene.registry.events.off(`changedata-${SELECTED}`, draw);
    scene.registry.events.off(`changedata-${KINDS}`, draw);
    scene.scale.off("resize", draw);
  });
  redraw();
}
