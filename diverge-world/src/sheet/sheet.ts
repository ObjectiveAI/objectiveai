// The bottom sheet: one agent's or one tool's log, in HTML over the map.

import type { Subject } from "../world/model";
import { Fold } from "./fold";
import { type Entry, type Following, follow } from "./log";
import { ToolFold } from "./tool_fold";
import "./sheet.css";

/** The sheet's height when it opens, and the least and most it may be dragged to, in hundredths of the window. */
const HEIGHT = 40;
const LEAST = 20;
const MOST = 90;

/** How near the bottom, in pixels, still counts as at it. */
const PINNED = 2;

/** What the sheet is opened on: whose log, its label, and the image names for an agent's tools. */
export type Opened = { subject: Subject; label: string; images: Map<string, string> };

/** A template and an index, as the header shows them: the template's first eight characters. */
const short = (template: string, index: number) => `${template.slice(0, 8)} #${index}`;

/**
 * The header's second line: an agent's or a created tool's template
 * and index; a connected tool's daemon; a dependency tool's template
 * and whose it is.
 */
function subline(subject: Subject): string {
  if ("account" in subject) return subject.account.issuer;
  if ("admin" in subject) return `admin at ${subject.admin.issuer}`;
  if ("agent" in subject) return "template" in subject.agent ? short(subject.agent.template, subject.agent.index) : subject.agent.name;
  if ("standalone_tool" in subject) {
    const ref = subject.standalone_tool;
    if ("daemon" in ref) return `from ${ref.daemon}`;
    if ("template" in ref) return short(ref.template, ref.index);
    return ref.name;
  }
  const dependency = subject.dependency_tool;
  const owner = dependency.owner;
  const whose = owner === "global" ? "global" : "agent" in owner ? ("template" in owner.agent ? short(owner.agent.template, owner.agent.index) : owner.agent.name) : `template ${owner.agent_template.template.slice(0, 8)}`;
  return `${dependency.template.slice(0, 8)} · ${whose}`;
}

/**
 * The sheet: closed until `open`, one log at a time.
 *
 * The rules:
 *
 * 1. It lies along the bottom of the window, the full width, `HEIGHT`
 *    hundredths tall; its handle drags it between `LEAST` and `MOST`.
 * 2. Its header names whose log it is (`subline`), and whether the
 *    agent's run is up or the tool has an ask in flight; ✕ closes it
 *    and tells `closed`.
 * 3. Opening follows the subject's log (`follow`, the root `logs`
 *    read) into `Fold` for an agent, `ToolFold` for a tool or an
 *    account (whose log is commands alone), each entry
 *    unwrapped from its kind; closing stops it and empties the body.
 *    Opening while open closes first.
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
  private fold: Fold | ToolFold | null = null;
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
    this.sub.textContent = subline(opened.subject);
    this.live.classList.remove("on");
    this.pinned = true;
    this.root.style.height = `${this.height}vh`;
    this.root.hidden = false;
    const live = (on: boolean) => this.live.classList.toggle("on", on);
    if ("agent" in opened.subject) {
      const fold = new Fold(this.body, opened.images, () => this.stick(), live);
      this.fold = fold;
      this.following = follow(opened.subject, (entry) => "agent" in entry && fold.push(entry.agent), (why) => fold.note(why));
    } else {
      const fold = new ToolFold(this.body, () => this.stick(), live);
      this.fold = fold;
      const tool = (entry: Entry) =>
        "standalone_tool" in entry
          ? entry.standalone_tool
          : "dependency_tool" in entry
            ? entry.dependency_tool
            : "account" in entry
              ? entry.account
              : "admin" in entry
                ? entry.admin
                : null;
      this.following = follow(opened.subject, (entry) => {
        const item = tool(entry);
        if (item) fold.push(item);
      }, (why) => fold.note(why));
    }
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
