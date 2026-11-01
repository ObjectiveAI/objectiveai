// The transfers strip: every download and upload under way, in the
// window's corner, whatever sheet is open or none.

import { type Outcome, type Running, cancel } from "./transfer";

/** How long a finished transfer stays shown, in milliseconds. */
const LINGER = 5000;

/** A byte count in words: `532 B`, `1.4 KB`, `12.0 MB`, `108 GB`. */
export function bytes(n: number): string {
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

/** An outcome in words; `null` for one that needs none. */
function words(outcome: Outcome, direction: "down" | "up"): string | null {
  switch (outcome.kind) {
    case "done":
      return direction === "down" ? `saved, ${bytes(outcome.bytes)}` : `uploaded, ${bytes(outcome.bytes)}`;
    case "cancelled":
      return null;
    case "not_found":
      return "not found — a dependency tool must be running";
    case "forbidden":
      return "this account may not";
    case "held":
      return "in use by a running container, or another transfer";
    case "exists":
      return `already there: ${outcome.path}`;
    case "failed":
      return outcome.why;
  }
}

/**
 * The strip.
 *
 * The rules:
 *
 * 1. It stands over the window's bottom right, above the sheet, and
 *    shows only while it holds a transfer.
 * 2. Each transfer is a line: its direction and name, a bar of the
 *    bytes moved out of the total where the total is known, the bytes
 *    in words, and ✕.
 * 3. ✕ stops a transfer under way, and takes a finished one away.
 * 4. A transfer that ends says how; one that finished goes after
 *    `LINGER`, one that was cancelled at once, and any other stays
 *    until ✕.
 */
class Strip {
  private readonly root: HTMLElement;

  constructor() {
    this.root = document.createElement("section");
    this.root.className = "transfers";
    this.root.hidden = true;
    document.body.append(this.root);
  }

  /**
   * Start a transfer and show it: `begin` is handed the progress to
   * tell and starts the transfer; `after` hears how it ended.
   */
  start(name: string, direction: "down" | "up", total: number | null, begin: (progress: (moved: number) => void) => Running, after?: (outcome: Outcome) => void): void {
    const line = document.createElement("div");
    line.className = "transfer";
    const label = document.createElement("span");
    label.className = "what";
    label.textContent = `${direction === "down" ? "⤓" : "⤒"} ${name}`;
    label.title = name;
    const bar = document.createElement("span");
    bar.className = "bar";
    const fill = document.createElement("span");
    bar.append(fill);
    bar.hidden = total === null;
    const said = document.createElement("span");
    said.className = "said";
    said.textContent = "…";
    const exit = document.createElement("button");
    exit.type = "button";
    exit.className = "exit";
    exit.textContent = "✕";
    exit.title = "Stop";
    line.append(label, exit, bar, said);
    this.root.append(line);
    this.root.hidden = false;
    // Rule 2.
    const progress = (moved: number) => {
      said.textContent = total === null ? bytes(moved) : `${bytes(moved)} of ${bytes(total)}`;
      if (total) fill.style.width = `${Math.min(100, (moved / total) * 100)}%`;
    };
    const running = begin(progress);
    let over = false;
    const remove = () => {
      line.remove();
      this.root.hidden = this.root.childElementCount === 0;
    };
    // Rule 3.
    exit.addEventListener("click", () => (over ? remove() : cancel(running.id)));
    // Rule 4.
    void running.ended.then((outcome) => {
      over = true;
      exit.title = "Dismiss";
      after?.(outcome);
      const told = words(outcome, direction);
      if (told === null) return remove();
      said.textContent = told;
      line.classList.add(outcome.kind === "done" ? "done" : "failed");
      if (outcome.kind === "done") {
        fill.style.width = "100%";
        window.setTimeout(remove, LINGER);
      }
    });
  }
}

/** The one strip. */
export const strip = new Strip();
