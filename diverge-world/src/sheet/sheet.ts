// The bottom sheet: one entity's log, files and console, in HTML over the map.

import type { Entity, Subject } from "../world/model";
import { type Container, Console } from "./console";
import { Files } from "./files";
import { Fold } from "./fold";
import { type Entry, type Following, follow } from "./log";
import { ToolFold } from "./tool_fold";
import "./sheet.css";

/** The sheet's height when it opens, and the least and most it may be dragged to, in hundredths of the window. */
const HEIGHT = 40;
const LEAST = 20;
const MOST = 90;

/** What the sheet is opened on: the entity, and the image names for an agent's tools. */
export type Opened = { entity: Entity; images: Map<string, string> };

/** The sheet's three tabs. */
type Tab = "logs" | "files" | "console";

/** The tabs an entity has, in order: logs with a log, a filesystem with files, a console with a container. */
function tabsOf(entity: Entity): Tab[] {
  const tabs: Tab[] = [];
  if (entity.subject) tabs.push("logs");
  if (entity.files) tabs.push("files");
  if (container(entity)) tabs.push("console");
  return tabs;
}

/** The entity's container, when it runs in one: an agent, a tool, a dependency tool — never a volume. */
function container(entity: Entity): Container | null {
  return entity.files && !("volume" in entity.files) ? entity.files : null;
}

/** The tab last chosen, for the page: the next sheet opens on it when it has it. */
let chosen: Tab = "logs";

/** A template and an index, as the header shows them: the template's first eight characters. */
const short = (template: string, index: number) => `${template.slice(0, 8)} #${index}`;

/**
 * The header's second line: an agent's or a created tool's template
 * and index; a connected tool's daemon; a dependency tool's template
 * and whose it is.
 */
function subline(subject: Subject): string {
  switch (subject.type) {
    case "account":
      return subject.issuer;
    case "admin":
      return `admin at ${subject.issuer}`;
    case "agent":
    case "standalone_tool":
      if ("daemon" in subject) return `from ${subject.daemon}`;
      if ("template" in subject) return short(subject.template, subject.index);
      return subject.name;
    case "dependency_tool": {
      const owner = subject.owner;
      const whose = owner === "global" ? "global" : "agent" in owner ? ("template" in owner.agent ? short(owner.agent.template, owner.agent.index) : owner.agent.name) : `template ${owner.agent_template.template.slice(0, 8)}`;
      return `${subject.template.slice(0, 8)} · ${whose}`;
    }
  }
}

/**
 * The sheet: closed until `open`, one entity at a time.
 *
 * The rules:
 *
 * 1. It lies along the bottom of the window, the full width, `HEIGHT`
 *    hundredths tall; its handle drags it between `LEAST` and `MOST`.
 * 2. Its header names the entity (`subline`, else its detail), and
 *    whether the agent's run is up or the tool has an ask in flight;
 *    its tabs, `logs`, `filesystem` and `console`, those it has, shown
 *    however many it has, one among them; and ✕, which closes it and tells
 *    `closed`.
 * 3. An entity with a log (`subject`) has the logs tab, one with files
 *    (`files`) the filesystem tab, one with a container (an agent, a
 *    tool, a dependency tool) the console tab (`tabsOf`). It opens on
 *    the tab last chosen (`chosen`) if it has it, else on the first it
 *    has. A tab's read starts the first time it is shown and runs until
 *    the sheet closes; closing stops every one and empties every one.
 *    Opening while open closes first.
 * 4. The logs tab follows the log (`follow`, the root `logs` read)
 *    into `Fold` for an agent, `ToolFold` for a tool or an account
 *    (whose log is commands alone), each entry unwrapped from its
 *    kind. It is held at its bottom while the history comes in: it
 *    cannot be scrolled (`loading`) until the read is live, or ends.
 *    Then it stays tethered to its bottom as it grows, and holds still
 *    once scrolled up, until scrolled back down. The tether is the
 *    page's own: the body is a reversed column, whose resting place is
 *    its bottom, and the browser's scroll anchoring keeps what is read
 *    in place above it.
 * 5. The filesystem tab is `Files`, scrolled from its top.
 * 6. The console tab is `Console`: one shell, opened the first time
 *    the tab is shown and closed — killed — with the sheet, or when it
 *    swaps to another entity; the keys go to it while it is shown.
 * 7. Nothing pressed on the sheet reaches the map beneath.
 */
export class Sheet {
  private readonly root: HTMLElement;
  private readonly title: HTMLElement;
  private readonly sub: HTMLElement;
  private readonly live: HTMLElement;
  private readonly tabs: HTMLElement;
  private readonly tab: Record<Tab, HTMLButtonElement>;
  private readonly body: HTMLElement;
  private readonly log: HTMLElement;
  private readonly pane: HTMLElement;
  private readonly shell: HTMLElement;
  private opened: Opened | null = null;
  private following: Following | null = null;
  private fold: Fold | ToolFold | null = null;
  private files: Files | null = null;
  private console: Console | null = null;
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
    // Rule 2.
    this.tabs = document.createElement("span");
    this.tabs.className = "tabs";
    const tab = (which: Tab, words: string) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "tab";
      button.textContent = words;
      button.addEventListener("click", () => {
        chosen = which;
        this.show(which);
      });
      return button;
    };
    this.tab = { logs: tab("logs", "logs"), files: tab("files", "filesystem"), console: tab("console", "console") };
    this.tabs.append(this.tab.logs, this.tab.files, this.tab.console);
    const exit = document.createElement("button");
    exit.className = "exit";
    exit.type = "button";
    exit.textContent = "✕";
    exit.title = "Close";
    exit.addEventListener("click", () => {
      this.close();
      this.closed();
    });
    head.append(this.live, this.title, this.sub, this.tabs, exit);
    this.body = document.createElement("div");
    this.body.className = "body";
    // Rule 4.
    this.log = document.createElement("div");
    this.log.className = "log";
    this.body.append(this.log);
    // Rule 5.
    this.pane = document.createElement("div");
    this.pane.className = "files";
    // Rule 6.
    this.shell = document.createElement("div");
    this.shell.className = "console";
    this.root.append(handle, head, this.body, this.pane, this.shell);
    this.drag(handle);
    document.body.append(this.root);
  }

  /** Rule 3. */
  open(opened: Opened): void {
    this.close();
    const { entity } = opened;
    this.opened = opened;
    this.title.textContent = entity.label;
    this.sub.textContent = entity.subject ? subline(entity.subject) : entity.detail;
    this.live.classList.remove("on");
    const tabs = tabsOf(entity);
    this.tabs.hidden = tabs.length === 0;
    for (const which of ["logs", "files", "console"] as const) this.tab[which].hidden = !tabs.includes(which);
    this.root.style.height = `${this.height}vh`;
    this.root.hidden = false;
    this.show(tabs.includes(chosen) ? chosen : (tabs[0] ?? "logs"));
  }

  /** Rule 3: stop, empty and hide. Closing a closed sheet does nothing. */
  close(): void {
    this.following?.stop();
    this.following = null;
    this.fold?.dispose();
    this.fold = null;
    this.log.replaceChildren();
    this.files?.dispose();
    this.files = null;
    this.console?.dispose();
    this.console = null;
    this.opened = null;
    this.root.hidden = true;
  }

  /** Rule 3: show one tab, starting its read the first time. */
  private show(which: Tab): void {
    const opened = this.opened;
    if (!opened) return;
    this.tab.logs.classList.toggle("on", which === "logs");
    this.tab.files.classList.toggle("on", which === "files");
    this.tab.console.classList.toggle("on", which === "console");
    this.body.hidden = which !== "logs";
    this.pane.hidden = which !== "files";
    this.shell.hidden = which !== "console";
    const { subject, files } = opened.entity;
    if (which === "logs" && subject && !this.following) this.logs(subject, opened.images);
    if (which === "files" && files && !this.files) this.files = new Files(this.pane, files);
    if (which === "console") {
      const shell = container(opened.entity);
      if (shell && !this.console) this.console = new Console(this.shell, shell);
      this.console?.shown();
    }
  }

  /** Rule 4. */
  private logs(subject: Subject, images: Map<string, string>): void {
    this.body.classList.add("loading");
    const live = (on: boolean) => this.live.classList.toggle("on", on);
    const loaded = () => this.body.classList.remove("loading");
    if (subject.type === "agent") {
      const fold = new Fold(this.log, images, live);
      this.fold = fold;
      this.following = follow(subject, (entry) => "agent" in entry && fold.push(entry.agent), (why) => {
        loaded();
        fold.note(why);
      }, loaded);
    } else {
      const fold = new ToolFold(this.log, live);
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
      this.following = follow(subject, (entry) => {
        const item = tool(entry);
        if (item) fold.push(item);
      }, (why) => {
        loaded();
        fold.note(why);
      }, loaded);
    }
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
