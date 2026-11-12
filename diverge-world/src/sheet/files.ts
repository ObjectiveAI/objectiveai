// The filesystem tab: one entity's files as a tree of columns, kept
// current as its read tells.

import type { shared } from "@diverge/sdk";
import type { Route } from "../lib/wire";
import type { Files as Target } from "../world/model";
import { type Root, readTree } from "./filetree";
import type { Following } from "./log";
import { bytes, strip } from "./strip";
import { type Drag, download, onDrag, upload } from "./transfer";

type Node = shared.filetree.response.Node;

/** The columns, in order; each one a header that sorts by it. */
const COLUMNS = ["name", "size", "created", "modified"] as const;
type Column = (typeof COLUMNS)[number];

/** How long a changed row glows, in milliseconds; the CSS animation's own length. */
const FLASH = 1200;

/** Unix seconds as a local time: `2026-07-14 09:31`. */
function when(seconds: number): string {
  const d = new Date(seconds * 1000);
  const pad = (x: number) => x.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** A node's value in a column: a size only for a file; `null` where it has none. */
function value(node: Node, column: Column): string | number | null {
  switch (column) {
    case "name":
      return node.name;
    case "size":
      return node.type === "file" ? node.size : null;
    case "created":
      return node.created_at;
    case "modified":
      return node.modified_at;
  }
}

/** A path as one key: its components joined by NUL, which no name holds. */
const keyOf = (path: string[]) => path.join("\0");

/**
 * The filesystem tab, reading from the moment it is made until
 * `dispose`.
 *
 * The rules:
 *
 * 1. A header row stands over the rows, every row on the same columns
 *    — name, size, created, modified — however deep it is.
 * 2. Directories come before everything else at every level; within
 *    each group the rows go by the sorted column, then by name.
 *    Pressing a header sorts by it up, then down, then not at all; one
 *    column at a time. A value missing sorts last either way and shows
 *    as `—`.
 * 3. Every directory starts closed; pressing one opens or closes it.
 *    What is open stays open through every change, a snapshot sent
 *    again among them, and an open directory that is gone is forgotten.
 *    An open empty directory says `empty`.
 * 4. Only the rows shown are drawn, again on the frame after anything
 *    changes, once however many changes came.
 * 5. A path a change names glows for `FLASH` (a closed directory glows
 *    for what changed beneath it), from when it changed, however many
 *    times the rows are drawn meanwhile.
 * 6. A container's directory whose changes are not watched is dim, as
 *    possibly stale; a volume's tree is a snapshot, so none is.
 * 7. Until the first tree, it says `reading…`; an empty tree says `no
 *    files`; a read that ends says why, under the last tree if there
 *    was one.
 * 8. A volume's header holds ↻, which asks again.
 * 9. Every row lights while it is pointed at. Right-clicking one opens
 *    its menu (`menu`) at the pointer: `Download`, or `Download folder`
 *    for a directory, saves it where the system's dialog says
 *    (`download`), its total the tree's sizes. The menu closes on a
 *    choice, a press anywhere else, Escape, or a scroll.
 * 10. Files dragged over the pane are aimed at a directory: the one
 *    pointed at, a file's own directory, or the root on the pane's
 *    empty space. That directory's row and what is open beneath it
 *    glow, and a line says where they go. Dropped there, they are
 *    uploaded (`upload`); a volume's tree is read again once they
 *    land. Outside the pane, or while the pane is not shown, nothing
 *    is aimed at.
 * 11. Transfers run in the strip (`strip`), which outlives the sheet.
 *    Reads and transfers alike are served on the daemon `route`
 *    reaches.
 * 12. Clicking a row, or right-clicking it, selects it: one row at a
 *    time, held lit more strongly than the pointer's light, through
 *    every change, until another is chosen or the pane's empty space
 *    is clicked. A directory clicked also opens or closes.
 */
export class Files {
  private readonly head: HTMLElement;
  private readonly rows: HTMLElement;
  private readonly note: HTMLElement;
  private reading: Following | null = null;
  private root: Root | null = null;
  private ended: string | null = null;
  private readonly open = new Set<string>();
  private readonly flashes = new Map<string, number>();
  private sort: { column: Column; down: boolean } | null = null;
  private frame = 0;
  private timer = 0;
  private readonly banner: HTMLElement;
  private dropping: string[] | null = null;
  private unlisten: (() => void) | null = null;
  private disposed = false;
  private selected: string | null = null;

  constructor(
    private readonly pane: HTMLElement,
    private readonly target: Target,
    private readonly route: Route,
  ) {
    this.head = document.createElement("div");
    this.head.className = "row head";
    this.rows = document.createElement("div");
    this.note = document.createElement("div");
    this.note.className = "note";
    this.banner = document.createElement("div");
    this.banner.className = "aim";
    this.banner.hidden = true;
    this.pane.append(this.head, this.banner, this.rows, this.note);
    // Rule 12: a click on no row lets go of the selection.
    this.pane.addEventListener("click", (event) => {
      if (event.target instanceof Element && !event.target.closest(".row[data-key]")) this.select(null);
    });
    this.heads();
    this.read();
    // Rule 10.
    void onDrag((drag) => this.dragged(drag)).then((unlisten) => {
      if (this.disposed) unlisten();
      else this.unlisten = unlisten;
    });
  }

  /** Stop reading and empty the pane. */
  dispose(): void {
    this.disposed = true;
    this.unlisten?.();
    this.pane.classList.remove("aimed");
    this.reading?.stop();
    this.reading = null;
    cancelAnimationFrame(this.frame);
    clearTimeout(this.timer);
    this.pane.replaceChildren();
  }

  /** Rule 8: start reading, or read again. */
  private read(): void {
    this.reading?.stop();
    this.ended = null;
    this.reading = readTree(
      this.target,
      this.route,
      (root, touched) => {
        this.root = root;
        const now = performance.now();
        for (const path of touched) this.flashes.set(keyOf(path), now);
        this.later();
      },
      (why) => {
        this.ended = why;
        this.later();
      },
    );
    this.later();
  }

  /** Rules 1, 2 and 8: the header row. */
  private heads(): void {
    const cells = COLUMNS.map((column) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "sort";
      const on = this.sort?.column === column;
      button.classList.toggle("on", on);
      button.textContent = column + (on ? (this.sort?.down ? " ↓" : " ↑") : "");
      button.addEventListener("click", () => {
        this.sort = !on ? { column, down: false } : !this.sort?.down ? { column, down: true } : null;
        this.heads();
        this.later();
      });
      return button;
    });
    const first = document.createElement("span");
    first.className = "cell";
    first.append(cells[0]);
    if ("volume" in this.target) {
      const again = document.createElement("button");
      again.type = "button";
      again.className = "again";
      again.textContent = "↻";
      again.title = "Read again";
      again.addEventListener("click", () => this.read());
      first.append(again);
    }
    const rest = cells.slice(1).map((button) => {
      const span = document.createElement("span");
      span.className = "cell";
      span.append(button);
      return span;
    });
    this.head.replaceChildren(first, ...rest);
  }

  /** Rule 10: a drag over the window, aimed or not. */
  private dragged(drag: Drag): void {
    if (drag.kind === "leave") return this.aim(null);
    const into = this.aimedAt(drag.x, drag.y);
    if (drag.kind === "over") return this.aim(into);
    this.aim(null);
    if (into === null || drag.paths.length === 0) return;
    const name = drag.paths.length === 1 ? (drag.paths[0].split(/[\\/]/).pop() ?? drag.paths[0]) : `${drag.paths.length} items`;
    strip.start(`${name} → /${into.join("/")}`, "up", null, (progress) => upload(this.target, this.route, into, drag.paths, progress), (outcome) => {
      if (!this.disposed && outcome.kind === "done" && "volume" in this.target) this.read();
    });
  }

  /** Rule 10: the directory under a point, or `null` off the pane. */
  private aimedAt(x: number, y: number): string[] | null {
    if (this.disposed || this.pane.hidden) return null;
    const under = document.elementFromPoint(x, y);
    if (!under || !this.pane.contains(under)) return null;
    const row = under.closest<HTMLElement>(".row[data-key]");
    if (!row?.dataset.key) return [];
    const path = row.dataset.key.split("\0");
    return row.dataset.kind === "directory" ? path : path.slice(0, -1);
  }

  /** Rule 10: aim at a directory, or at nothing. */
  private aim(into: string[] | null): void {
    const same = into === null ? this.dropping === null : this.dropping !== null && keyOf(into) === keyOf(this.dropping) && into.length === this.dropping.length;
    if (same) return;
    this.dropping = into;
    this.banner.hidden = into === null;
    this.banner.textContent = into === null ? "" : `upload to /${into.join("/")}`;
    this.pane.classList.toggle("aimed", into !== null && into.length === 0);
    this.later();
  }

  /** Rule 12. */
  private select(key: string | null): void {
    if (this.selected === key) return;
    this.selected = key;
    this.later();
  }

  /** Rule 9. */
  private download(node: Node, path: string[]): void {
    const total = size(node);
    strip.start(path.join("/"), "down", total, (progress) => download(this.target, this.route, path, node.type === "directory", progress));
  }

  /** Rule 4: draw on the next frame, once. */
  private later(): void {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.draw();
    });
  }

  /** Rule 2. */
  private order(nodes: Node[]): Node[] {
    const sort = this.sort;
    return [...nodes].sort((a, b) => {
      const dirs = Number(b.type === "directory") - Number(a.type === "directory");
      if (dirs !== 0) return dirs;
      if (sort) {
        const x = value(a, sort.column);
        const y = value(b, sort.column);
        if (x !== y) {
          if (x === null) return 1;
          if (y === null) return -1;
          const by = typeof x === "number" && typeof y === "number" ? x - y : String(x).localeCompare(String(y));
          if (by !== 0) return sort.down ? -by : by;
        }
      }
      return a.name.localeCompare(b.name);
    });
  }

  /** Rules 3 to 7. */
  private draw(): void {
    const root = this.root;
    const now = performance.now();
    for (const [key, at] of this.flashes) if (now - at >= FLASH) this.flashes.delete(key);
    // Rule 3: an open directory that is gone is forgotten.
    if (root) for (const key of this.open) if (!isDirectory(root.children, key.split("\0"))) this.open.delete(key);
    const drawn: HTMLElement[] = [];
    const walk = (nodes: Node[], path: string[]) => {
      for (const node of this.order(nodes)) {
        const at = [...path, node.name];
        const key = keyOf(at);
        const opened = node.type === "directory" && this.open.has(key);
        drawn.push(this.row(node, at, key, opened, now));
        if (node.type === "directory" && opened) {
          if (node.children.length === 0) drawn.push(empty(at.length));
          else walk(node.children, at);
        }
      }
    };
    if (root) walk(root.children, []);
    this.rows.replaceChildren(...drawn);
    // Rule 7.
    this.note.textContent = this.ended ?? (root === null ? "reading…" : root.children.length === 0 ? "no files" : "");
    this.note.hidden = this.note.textContent === "";
    this.note.classList.toggle("failed", this.ended !== null);
    if (this.flashes.size > 0) {
      const next = Math.min(...this.flashes.values()) + FLASH - now;
      clearTimeout(this.timer);
      this.timer = window.setTimeout(() => this.later(), Math.max(0, next));
    }
  }

  /** One row: its caret and name, indented, then its size and times. */
  private row(node: Node, at: string[], key: string, opened: boolean, now: number): HTMLElement {
    const depth = at.length - 1;
    const row = document.createElement("div");
    row.className = `row ${node.type}`;
    row.dataset.key = key;
    row.dataset.kind = node.type;
    const name = document.createElement("span");
    name.className = "cell name";
    name.style.paddingLeft = `calc(var(--space-3) + ${depth} * var(--space-4))`;
    const caret = document.createElement("span");
    caret.className = "caret";
    caret.textContent = node.type === "directory" ? (opened ? "▾" : "▸") : "";
    const label = document.createElement("span");
    label.textContent = node.type === "directory" ? `${node.name}/` : node.type === "symlink" ? `${node.name} → ${node.path.join("/")}` : node.name;
    name.append(caret, label);
    // Rule 9.
    row.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      // Rule 12.
      this.select(key);
      menu(event.clientX, event.clientY, [{ words: node.type === "directory" ? "Download folder" : "Download", act: () => this.download(node, at) }]);
    });
    row.append(name, cell(node.type === "file" && node.size !== null ? bytes(node.size) : null), cell(node.created_at === null ? null : when(node.created_at)), cell(node.modified_at === null ? null : when(node.modified_at)));
    // Rule 12, and rule 3 for a directory.
    row.addEventListener("click", () => {
      this.select(key);
      if (node.type !== "directory") return;
      if (this.open.has(key)) this.open.delete(key);
      else this.open.add(key);
      this.later();
    });
    if (key === this.selected) row.classList.add("selected");
    if (node.type === "directory") {
      // Rule 6.
      if (!node.changes && !("volume" in this.target)) {
        row.classList.add("stale");
        row.title = "not watched — may be stale";
      }
    }
    // Rule 5.
    let since = this.flashes.get(key);
    if (since === undefined && node.type === "directory" && !opened) {
      for (const [k, t] of this.flashes) if (k.startsWith(`${key}\0`)) since = Math.max(since ?? t, t);
    }
    if (since !== undefined) {
      row.classList.add("flash");
      row.style.animationDelay = `${-(now - since)}ms`;
    }
    // Rule 10.
    if (this.dropping !== null && this.dropping.length > 0) {
      const aimed = keyOf(this.dropping);
      if (key === aimed || key.startsWith(`${aimed}\0`)) row.classList.add("aimed");
    }
    return row;
  }
}

/** A value cell: the value, or a dim `—`. */
function cell(text: string | null): HTMLElement {
  const span = document.createElement("span");
  span.className = text === null ? "cell none" : "cell";
  span.textContent = text ?? "—";
  return span;
}

/** Rule 3: the row under an open empty directory. */
function empty(depth: number): HTMLElement {
  const row = document.createElement("div");
  row.className = "row empty";
  const span = document.createElement("span");
  span.className = "cell name";
  // Past its caret: one step more.
  span.style.paddingLeft = `calc(var(--space-3) + ${depth + 1} * var(--space-4))`;
  span.textContent = "empty";
  row.append(span);
  return row;
}

/** One thing a row's menu offers: its words, and what choosing it does. */
type Choice = { words: string; act: () => void };

/** The menu open now, and how it closes; at most one at a time. */
let closing: (() => void) | null = null;

/** Rule 9: a row's menu, at the pointer, kept inside the window. */
function menu(x: number, y: number, choices: Choice[]): void {
  closing?.();
  const box = document.createElement("div");
  box.className = "files-menu";
  for (const choice of choices) {
    const item = document.createElement("button");
    item.type = "button";
    item.textContent = choice.words;
    item.addEventListener("click", () => {
      close();
      choice.act();
    });
    box.append(item);
  }
  document.body.append(box);
  const { width, height } = box.getBoundingClientRect();
  box.style.left = `${Math.max(0, Math.min(x, window.innerWidth - width - 4))}px`;
  box.style.top = `${Math.max(0, Math.min(y, window.innerHeight - height - 4))}px`;
  const away = (event: Event) => {
    if (!(event.target instanceof Element) || !box.contains(event.target)) close();
  };
  const key = (event: KeyboardEvent) => {
    if (event.key === "Escape") close();
  };
  const close = () => {
    box.remove();
    document.removeEventListener("pointerdown", away, true);
    document.removeEventListener("keydown", key, true);
    document.removeEventListener("wheel", close, true);
    window.removeEventListener("blur", close);
    if (closing === close) closing = null;
  };
  document.addEventListener("pointerdown", away, true);
  document.addEventListener("keydown", key, true);
  document.addEventListener("wheel", close, true);
  window.addEventListener("blur", close);
  closing = close;
}

/** Rule 9: a file's size, or every file's beneath a directory; `null` where one is not known. */
function size(node: Node): number | null {
  if (node.type === "file") return node.size;
  if (node.type === "symlink") return 0;
  let total = 0;
  for (const child of node.children) {
    const own = size(child);
    if (own === null) return null;
    total += own;
  }
  return total;
}

/** Whether a directory stands at `path` in the tree. */
function isDirectory(nodes: Node[], path: string[]): boolean {
  let level = nodes;
  for (let i = 0; i < path.length; i++) {
    const node = level.find((n) => n.name === path[i]);
    if (!node || node.type !== "directory") return false;
    level = node.children;
  }
  return true;
}
