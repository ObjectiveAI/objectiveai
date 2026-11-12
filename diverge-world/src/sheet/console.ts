// The console tab: an interactive shell in one entity's container, on
// a terminal drawn by xterm.

import { daemon } from "@diverge/sdk";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import { css } from "../lib/theme";
import { handle, type Route, relayed } from "../lib/wire";
import type { Files } from "../world/model";

/** A container's files, which are whose shell it is: an agent's, a tool's, a dependency tool's. */
export type Container = Exclude<Files, { volume: unknown }>;

/** One answer of a terminal, as the three families answer it alike. */
type Item = daemon.endpoints.agents.terminal.server.response.Item;

/** The way into an open terminal, as the three families hand it back alike: each its own class, one shape. */
type Way = Pick<daemon.endpoints.agents.terminal.client.Terminal, "stdin" | "resize" | "close">;

/** How long the pane's size holds still before the terminal is told it, in milliseconds. */
const SETTLE = 120;

/** The fewest characters across and lines down a terminal is opened or resized to. */
const LEAST = { cols: 20, rows: 4 };

/** Open a terminal, by the family the entity is of, on the daemon `route` reaches. */
function open(container: Container, route: Route, size: { cols: number; rows: number }): Promise<[AsyncIterable<Item>, Way]> {
  if ("agent" in container) return daemon.endpoints.agents.terminal.client.execute(handle, { agent: container.agent, ...size, ...relayed(route) });
  if ("tool" in container) return daemon.endpoints.standalone_tools.terminal.client.execute(handle, { tool: container.tool, ...size, ...relayed(route) });
  return daemon.endpoints.dependency_tools.terminal.client.execute(handle, { dependency: container.dependency, ...size, ...relayed(route) });
}

/** The theme's mono stack (`--font-mono`), as the terminal's font; the same stack if the page has none. */
const mono = () => getComputedStyle(document.documentElement).getPropertyValue("--font-mono").trim() || 'ui-monospace, "Cascadia Code", Consolas, monospace';

/**
 * The console tab: a shell from the moment it is made until `dispose`.
 *
 * The rules:
 *
 * 1. It is one terminal filling the pane, at the size the pane holds
 *    (`fit`); the shell is opened at that size, on the daemon `route`
 *    reaches (`relayed`), the container started if it was not up, and
 *    held up while the tab lives.
 * 2. What is typed or pasted goes to the shell's stdin at once, one
 *    write in flight at a time: what is typed meanwhile is held and
 *    sent as one write once the last is done, so nothing reorders and
 *    nothing waits on the output.
 * 3. What the shell prints is drawn as it comes.
 * 4. The pane's size, once it has held still for `SETTLE`, is the
 *    terminal's, and the shell is told when it changed; never smaller
 *    than `LEAST`, and not while the pane is hidden.
 * 5. The shell ending says how, dimmed; so does a container that is
 *    not found, forbidden, or failing. Then any key opens a new shell
 *    on a cleared screen.
 * 6. `dispose` closes the shell, which kills it, and empties the pane.
 *    Nothing is told after it.
 */
export class Console {
  private readonly term: Terminal;
  private readonly fit = new FitAddon();
  private readonly watching: ResizeObserver;
  private way: Way | null = null;
  private state: "opening" | "open" | "ended" = "opening";
  private held: Uint8Array[] = [];
  private writing = false;
  private size = { cols: 0, rows: 0 };
  private settle = 0;
  private disposed = false;
  private readonly encoder = new TextEncoder();

  constructor(
    private readonly pane: HTMLElement,
    private readonly container: Container,
    private readonly route: Route,
  ) {
    this.term = new Terminal({
      // The cursor blinks only for someone who did not ask for less motion.
      cursorBlink: !matchMedia("(prefers-reduced-motion: reduce)").matches,
      fontFamily: mono(),
      fontSize: 13,
      scrollback: 5000,
      theme: {
        background: css("surface"),
        foreground: css("text"),
        cursor: css("accent"),
        selectionBackground: css("accent-soft"),
      },
    });
    this.term.loadAddon(this.fit);
    this.term.open(this.pane);
    // Rule 2.
    this.term.onData((data) => this.typed(this.encoder.encode(data)));
    this.term.onBinary((data) => this.typed(Uint8Array.from(data, (c) => c.charCodeAt(0) & 0xff)));
    // Rule 4.
    this.watching = new ResizeObserver(() => {
      window.clearTimeout(this.settle);
      this.settle = window.setTimeout(() => this.resized(), SETTLE);
    });
    this.watching.observe(this.pane);
    this.measure();
    void this.start();
  }

  /** The tab shown again: the terminal takes the keys, and its size is checked. */
  shown(): void {
    this.resized();
    this.term.focus();
  }

  /** Rule 6. */
  dispose(): void {
    this.disposed = true;
    window.clearTimeout(this.settle);
    this.watching.disconnect();
    void this.way?.close();
    this.way = null;
    this.term.dispose();
    this.pane.replaceChildren();
  }

  /** Rule 4: the size the pane holds, at least `LEAST`. */
  private measure(): { cols: number; rows: number } | null {
    if (this.pane.offsetParent === null) return null;
    const proposed = this.fit.proposeDimensions();
    if (!proposed || !Number.isFinite(proposed.cols) || !Number.isFinite(proposed.rows)) return null;
    const size = { cols: Math.max(LEAST.cols, proposed.cols), rows: Math.max(LEAST.rows, proposed.rows) };
    if (size.cols !== this.term.cols || size.rows !== this.term.rows) this.term.resize(size.cols, size.rows);
    return size;
  }

  /** Rule 4: the shell told the new size. */
  private resized(): void {
    if (this.disposed) return;
    const size = this.measure();
    if (!size || (size.cols === this.size.cols && size.rows === this.size.rows)) return;
    this.size = size;
    if (this.state === "open") void this.way?.resize(size).catch(() => {});
  }

  /** Rules 1, 3 and 5: open a shell and draw what it prints until it ends. */
  private async start(): Promise<void> {
    this.state = "opening";
    const size = this.measure() ?? { cols: 80, rows: 24 };
    this.size = size;
    let output: AsyncIterable<Item>;
    try {
      const [stream, way] = await open(this.container, this.route, size);
      if (this.disposed) {
        void way.close();
        return;
      }
      output = stream;
      this.way = way;
      this.state = "open";
    } catch (error) {
      this.ended(error instanceof Error ? error.message : String(error));
      return;
    }
    // A size that changed while it opened.
    this.resized();
    this.flush();
    let why = "the connection ended";
    try {
      for await (const item of output) {
        if (this.disposed) return;
        if (typeof item === "string") why = item === "not_found" ? ("dependency" in this.container ? "not running" : "not found") : "forbidden";
        else if ("stdout" in item) this.term.write(item.stdout);
        else if ("exited" in item) why = `exited ${item.exited}`;
        else if ("signaled" in item) why = `killed by signal ${item.signaled}`;
        else why = typeof item.error === "string" ? item.error : JSON.stringify(item.error);
      }
    } catch (error) {
      why = error instanceof Error ? error.message : String(error);
    }
    if (!this.disposed) this.ended(why);
  }

  /** Rule 5. */
  private ended(why: string): void {
    if (this.disposed) return;
    this.state = "ended";
    this.way = null;
    this.held = [];
    this.term.write(`\r\n\x1b[2m[${why} — press any key for a new shell]\x1b[0m\r\n`);
  }

  /** Rule 2, and rule 5's key. */
  private typed(bytes: Uint8Array): void {
    if (this.disposed) return;
    if (this.state === "ended") {
      this.term.reset();
      void this.start();
      return;
    }
    this.held.push(bytes);
    this.flush();
  }

  /** Rule 2: what is held, as one write, once the last is done. */
  private flush(): void {
    const way = this.way;
    if (this.writing || !way || this.held.length === 0) return;
    const length = this.held.reduce((sum, piece) => sum + piece.length, 0);
    const bytes = new Uint8Array(length);
    let at = 0;
    for (const piece of this.held) {
      bytes.set(piece, at);
      at += piece.length;
    }
    this.held = [];
    this.writing = true;
    way
      .stdin(bytes)
      .catch(() => {})
      .finally(() => {
        this.writing = false;
        this.flush();
      });
  }
}
