// The bottom sheet: one agent's log, in HTML over the map.

import { Fold } from "./fold";
import { type AgentRef, type Following, follow } from "./log";
import "./sheet.css";

/** The sheet's height when it opens, and the least and most it may be dragged to, in hundredths of the window. */
const HEIGHT = 40;
const LEAST = 20;
const MOST = 90;

/** How near the bottom, in pixels, still counts as at it. */
const PINNED = 2;

/** What the sheet is opened on: the agent, its label, and the image names for its tools. */
export type Opened = { agent: AgentRef; label: string; images: Map<string, string> };

/**
 * The sheet: closed until `open`, one agent at a time.
 *
 * The rules:
 *
 * 1. It lies along the bottom of the window, the full width, `HEIGHT`
 *    hundredths tall; its handle drags it between `LEAST` and `MOST`.
 * 2. Its header names the agent, its template and index, and whether
 *    a run is up; ✕ closes it and tells `closed`.
 * 3. Opening follows the agent's log (`follow`) into the body
 *    (`Fold`); closing stops it and empties the body. Opening while
 *    open closes first.
 * 4. The body stays at its bottom as it grows while it was at its
 *    bottom — within `PINNED` pixels — and stays where it is when
 *    scrolled up.
 * 5. Nothing pressed on the sheet reaches the map beneath.
 */
export class Sheet {
  private readonly root: HTMLElement;
  private readonly title: HTMLElement;
  private readonly sub: HTMLElement;
  private readonly live: HTMLElement;
  private readonly body: HTMLElement;
  private following: Following | null = null;
  private fold: Fold | null = null;
  private pinned = true;
  private height = HEIGHT;

  constructor(private readonly closed: () => void) {
    this.root = document.createElement("section");
    this.root.className = "sheet";
    this.root.hidden = true;
    const handle = document.createElement("div");
    handle.className = "handle";
    const head = document.createElement("header");
    this.live = document.createElement("span");
    this.live.className = "live";
    this.title = document.createElement("span");
    this.title.className = "title";
    this.sub = document.createElement("span");
    this.sub.className = "sub";
    const exit = document.createElement("button");
    exit.className = "exit";
    exit.type = "button";
    exit.textContent = "✕";
    exit.title = "Close";
    // Rule 2.
    exit.addEventListener("click", () => {
      this.close();
      this.closed();
    });
    head.append(this.live, this.title, this.sub, exit);
    this.body = document.createElement("div");
    this.body.className = "body";
    // Rule 4.
    this.body.addEventListener("scroll", () => {
      this.pinned = this.body.scrollHeight - this.body.scrollTop - this.body.clientHeight < PINNED;
    });
    this.root.append(handle, head, this.body);
    this.drag(handle);
    document.body.append(this.root);
  }

  /** Rule 3. */
  open(opened: Opened): void {
    this.close();
    this.title.textContent = opened.label;
    this.sub.textContent = `${opened.agent.template.slice(0, 8)} #${opened.agent.index}`;
    this.live.classList.remove("on");
    this.pinned = true;
    this.root.style.height = `${this.height}vh`;
    this.root.hidden = false;
    const fold = new Fold(this.body, opened.images, () => this.stick(), (on) => this.live.classList.toggle("on", on));
    this.fold = fold;
    this.following = follow(opened.agent, (item) => fold.push(item), (why) => fold.note(why));
  }

  /** Rule 3: stop, empty and hide. Closing a closed sheet does nothing. */
  close(): void {
    this.following?.stop();
    this.following = null;
    this.fold?.dispose();
    this.fold = null;
    this.body.replaceChildren();
    this.root.hidden = true;
  }

  /** Rule 4: back to the bottom, if it was there. */
  private stick(): void {
    if (this.pinned) this.body.scrollTop = this.body.scrollHeight;
  }

  /** Rule 1: the handle's drag sets the height. */
  private drag(handle: HTMLElement): void {
    handle.addEventListener("pointerdown", (down) => {
      handle.setPointerCapture(down.pointerId);
      const move = (e: PointerEvent) => {
        const share = ((window.innerHeight - e.clientY) / window.innerHeight) * 100;
        this.height = Math.min(MOST, Math.max(LEAST, share));
        this.root.style.height = `${this.height}vh`;
      };
      const up = () => {
        handle.removeEventListener("pointermove", move);
        handle.removeEventListener("pointerup", up);
        handle.removeEventListener("pointercancel", up);
      };
      handle.addEventListener("pointermove", move);
      handle.addEventListener("pointerup", up);
      handle.addEventListener("pointercancel", up);
    });
  }
}
