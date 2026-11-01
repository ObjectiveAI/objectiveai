// The filesystem tab: one entity's files as a tree of columns, kept
// current as its read tells.

import type { shared } from "@diverge/sdk";
import type { Files as Target } from "../world/model";
import { type Root, readTree } from "./filetree";
import type { Following } from "./log";

type Node = shared.filetree.response.Node;

/** The columns, in order; each one a header that sorts by it. */
const COLUMNS = ["name", "size", "created", "modified"] as const;
type Column = (typeof COLUMNS)[number];

/** How long a changed row glows, in milliseconds; the CSS animation's own length. */
const FLASH = 1200;

/** A byte count in words: `532 B`, `1.4 KB`, `12.0 MB`, `108 GB`. */
function bytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = n;
  let unit = -1;
  do {
    value /= 1024;
    unit++;
  } while (value >= 1024 && unit < units.length - 1);
  return `${value >= 100 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

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

  constructor(
    private readonly pane: HTMLElement,
    private readonly target: Target,
  ) {
    this.head = document.createElement("div");
    this.head.className = "row head";
    this.rows = document.createElement("div");
    this.note = document.createElement("div");
    this.note.className = "note";
    this.pane.append(this.head, this.rows, this.note);
    this.heads();
    this.read();
  }

  /** Stop reading and empty the pane. */
  dispose(): void {
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
        drawn.push(this.row(node, at.length - 1, key, opened, now));
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
    if (this.flashes.size > 0) {
      const next = Math.min(...this.flashes.values()) + FLASH - now;
      clearTimeout(this.timer);
      this.timer = window.setTimeout(() => this.later(), Math.max(0, next));
    }
  }

  /** One row: its caret and name, indented, then its size and times. */
  private row(node: Node, depth: number, key: string, opened: boolean, now: number): HTMLElement {
    const row = document.createElement("div");
    row.className = `row ${node.type}`;
    const name = document.createElement("span");
    name.className = "cell name";
    name.style.paddingLeft = `${0.75 + depth * 1.25}rem`;
    const caret = document.createElement("span");
    caret.className = "caret";
    caret.textContent = node.type === "directory" ? (opened ? "▾" : "▸") : "";
    const label = document.createElement("span");
    label.textContent = node.type === "directory" ? `${node.name}/` : node.type === "symlink" ? `${node.name} → ${node.path.join("/")}` : node.name;
    name.append(caret, label);
    row.append(name, cell(node.type === "file" && node.size !== null ? bytes(node.size) : null), cell(node.created_at === null ? null : when(node.created_at)), cell(node.modified_at === null ? null : when(node.modified_at)));
    if (node.type === "directory") {
      row.addEventListener("click", () => {
        if (this.open.has(key)) this.open.delete(key);
        else this.open.add(key);
        this.later();
      });
      // Rule 6.
      if (!node.changes && !("volume" in this.target)) {
        row.classList.add("stale");
        row.title = "not watched — may be stale";
      }
    }
    // Rule 5.
    let at = this.flashes.get(key);
    if (at === undefined && node.type === "directory" && !opened) {
      for (const [k, t] of this.flashes) if (k.startsWith(`${key}\0`)) at = Math.max(at ?? t, t);
    }
    if (at !== undefined) {
      row.classList.add("flash");
      row.style.animationDelay = `${-(now - at)}ms`;
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
  span.style.paddingLeft = `${0.75 + depth * 1.25 + 1}rem`;
  span.textContent = "empty";
  row.append(span);
  return row;
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
