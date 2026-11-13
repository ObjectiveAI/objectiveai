// The frontend tab: the page an agent's or a tool's own program serves, in a frame,
// from a server the app's Rust runs for it on the loopback.

import { invoke } from "@tauri-apps/api/core";
import { type Route, relayed } from "../lib/wire";
import type { Frontend as Target } from "../world/model";

/** What the app's Rust answers an open with: the frontend's id, and the address the frame loads. */
type Opened = { id: number; url: string };

/**
 * The frontend tab: serving from the moment it is made until `dispose`.
 *
 * The rules:
 *
 * 1. Made, it asks the app's Rust to serve the program's frontend
 *    (`frontend_open`), on the daemon `route` reaches (`relayed`), and
 *    loads the address it answers in a frame filling the pane; every
 *    request the frame makes goes to the program through the daemon, a
 *    WebSocket's included.
 * 2. Above the frame, ↻ loads the frontend again from its start, and the
 *    address it is served from stands beside it.
 * 3. A frontend that will not open says why in the pane.
 * 4. `dispose` — the sheet closed, or swapped to another node — ends the
 *    frontend (`frontend_close`): every request and connection it had
 *    open ends, and so does what they held up. One disposed before it
 *    finished opening is ended as soon as it has.
 */
export class FrontendPane {
  private readonly frame: HTMLIFrameElement;
  private readonly where: HTMLElement;
  private opened: Opened | null = null;
  private disposed = false;

  constructor(
    private readonly pane: HTMLElement,
    target: Target,
    route: Route,
  ) {
    // Rule 2.
    const bar = document.createElement("div");
    bar.className = "bar";
    const reload = document.createElement("button");
    reload.type = "button";
    reload.textContent = "↻";
    reload.title = "Load the frontend again";
    reload.addEventListener("click", () => {
      if (this.opened) this.frame.src = this.opened.url;
    });
    this.where = document.createElement("span");
    this.where.className = "where";
    this.where.textContent = "opening…";
    bar.append(reload, this.where);
    // Rule 1.
    this.frame = document.createElement("iframe");
    this.frame.title = "The frontend";
    // A frontend may put text on the clipboard: its copy buttons work.
    this.frame.allow = "clipboard-write";
    this.pane.append(bar, this.frame);
    void invoke<Opened>("frontend_open", { target, ...relayed(route) })
      .then((opened) => {
        // Rule 4: disposed while it opened.
        if (this.disposed) {
          void invoke("frontend_close", { id: opened.id });
          return;
        }
        this.opened = opened;
        this.where.textContent = new URL(opened.url).origin;
        this.frame.src = opened.url;
      })
      .catch((error: unknown) => {
        // Rule 3.
        this.where.textContent = `could not open: ${error instanceof Error ? error.message : String(error)}`;
      });
  }

  /** Rule 4. */
  dispose(): void {
    this.disposed = true;
    if (this.opened) void invoke("frontend_close", { id: this.opened.id });
    this.opened = null;
    this.pane.replaceChildren();
  }
}
