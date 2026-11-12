// The outline: every node on the board, in HTML over the canvas — a card
// that floats wherever it is dragged, folds to its header or to the
// kinds' shapes, and docks down the window's left as a rail.

import type * as Phaser from "phaser";
import { type EdgeType, TYPES } from "../grid/edges";
import { FAMILY } from "../grid/palette";
import { KIND_ORDER } from "../scenes/kinds";
import { ARRANGE, type Bounds, FOCUS, HOVER, KINDS, type Kinds, LINES, type Lines, OUTLINE, OUTLINER, type Outline, SELECTED, STATUS, type Status } from "../scenes/status";
import { TABS } from "../tabs/tabs";
import type { Kind } from "../world/model";
import { glyph, kindIcon, lineIcon } from "./icons";
import "./outline.css";

/** The card's and the rail's width (the viewer's rail), and the shapes' column's. */
const WIDTH = 244;
const COLUMN = 46;

/** How far in from the window's edges a card stands, how near an edge it snaps there, and how near the left edge a drop docks it. */
const EDGE = 12;
const SNAP = 24;
const DOCK = 48;

/** How far, in pixels, a press on the header may move and still not be a drag. */
const CLICK = 6;

/** The families, in order, and the kinds in each: who acts, what they use, where it runs. */
const FAMILIES: [string, Kind[]][] = [
  ["who acts", ["account", "agent"]],
  ["what they use", ["standalone_tool", "connected_standalone_tool", "dependency_tool"]],
  ["where it runs", ["volume", "provider", "daemon"]],
];

/** The kinds in the families' order, for the chips. */
const ORDER: Kind[] = FAMILIES.flatMap(([, kinds]) => kinds);

/** Each type's name. */
const NAMES: Record<EdgeType, string> = { call: "calls", attachment: "attachments", creator: "creators" };

/** A kind's name, as the menus have it. */
const kindName = (kind: Kind) => KIND_ORDER.find(([k]) => k === kind)?.[2] ?? kind;

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

/** What the world has to say, or `null` when nothing. */
function words(status: Status | undefined): string | null {
  if (status === undefined) return null;
  if (status.kind === "waiting") return status.reason === null ? "Waiting for the daemon…" : `Waiting for the daemon… (${status.reason})`;
  return status.forbidden.length === 0 ? null : `Not allowed to list: ${status.forbidden.join(", ")}`;
}

/** A button with its classes, its label for those who cannot see its glyph, and what it marks for focus to come back to. */
function button(classes: string, label: string, focus: string, inner: string): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.className = classes;
  b.title = label;
  b.setAttribute("aria-label", label);
  b.dataset.focus = focus;
  b.innerHTML = inner;
  return b;
}

/** The form the outline takes: open; folded to its header; folded to the kinds' shapes. */
type Form = "open" | "header" | "shapes";

/**
 * Start the outline over `game`.
 *
 * The rules:
 *
 * 1. It is HTML over the canvas, under the tab strip and the sheet, on
 *    the theme's ground with the strong edge, so it takes focus and the
 *    keys as the rest of the page does. A press or a wheel on it never
 *    reaches the board; its bounds are told the board all the same
 *    (`OUTLINER`), with whether it is held.
 * 2. Open, it is a header — Outline, how many nodes, Arrange (`ARRANGE`),
 *    fold, dock — then what the world has to say (`STATUS`), quietly;
 *    then what to show on the board, as chips: each type of line with its
 *    sample and count (`LINES`), each kind by its shape (`KINDS`), pressed
 *    while shown; then every node shown, by family — who acts, what they
 *    use, where it runs — each family's head folding its rows away.
 * 3. A row is a node: its shape in its family's colour, its name. A node
 *    not bright is faint: its shape in the strong edge, its name in the
 *    quietest words. The node whose sheet is open (`SELECTED`) is bold on
 *    a raised band with the gold bar down its left.
 * 4. Pressing a row tells the board (`FOCUS`), which brings it to the
 *    middle of the view and opens its sheet; hovering or focusing it
 *    tells the board (`HOVER`), which focuses its lines. Under the mouse
 *    or the keys a row's band lightens and its words brighten — no box —
 *    and while the keys are in use the mouse marks nothing. The arrow
 *    keys move along the list; Enter presses.
 * 5. Folded, it is its header alone, or a column of the kinds' shapes,
 *    each still a chip that shows or hides its kind; the first button
 *    opens it again.
 * 6. Dragged by its header it floats anywhere in the window under the
 *    strip; let go near an edge, it snaps `EDGE` in; let go near the
 *    window's left (`DOCK`), where a dashed outline shows while it is
 *    held there, it docks as a rail the window's height, `WIDTH` wide —
 *    or `COLUMN` folded — and the sheet opens beside it. Dragged off the
 *    rail, or its float button pressed, it floats again; its dock button
 *    docks it.
 * 7. A floating card stops `EDGE` above the sheet when one is open, or
 *    the window's foot, and its list scrolls within what is left.
 * 8. It keeps no files: where it stands and how it is folded live for
 *    the page alone.
 */
export function outline(game: Phaser.Game): void {
  const registry = game.registry;
  let form: Form = "open";
  let docked = false;
  let left: number | null = null;
  let top = TABS + EDGE;
  let held = false;
  const folded = new Set<string>();
  // Rule 1.
  const root = document.createElement("aside");
  root.className = "outline";
  root.setAttribute("aria-label", "Outline");
  const zone = document.createElement("div");
  zone.className = "dock-zone";
  zone.textContent = "Drop to dock";
  document.body.append(zone, root);

  /** The sheet's top, if one is open, in the window's pixels. */
  const sheetTop = () => {
    let at: number | null = null;
    for (const sheet of document.querySelectorAll<HTMLElement>(".sheet")) if (!sheet.hidden) at = Math.min(at ?? Infinity, sheet.getBoundingClientRect().top);
    return at;
  };
  /** The card's width in its form. */
  const width = () => (form === "shapes" ? COLUMN : WIDTH);

  // Rules 6 and 7: where it stands, and how tall it may be.
  const place = () => {
    const w = width();
    root.dataset.form = form;
    root.dataset.docked = String(docked);
    root.classList.toggle("held", held);
    document.documentElement.style.setProperty("--outline-left", docked ? `${w}px` : "0px");
    if (docked) {
      Object.assign(root.style, { left: "0px", top: `${TABS}px`, width: `${w}px`, height: `${window.innerHeight - TABS}px`, maxHeight: "" });
    } else {
      const floor = (sheetTop() ?? window.innerHeight) - EDGE;
      left = Math.max(0, Math.min(window.innerWidth - w, left ?? window.innerWidth - w - EDGE));
      top = Math.max(TABS, Math.min(Math.max(TABS, floor - 40), top));
      Object.assign(root.style, { left: `${left}px`, top: `${top}px`, width: `${w}px`, height: "", maxHeight: `${Math.max(40, floor - top)}px` });
    }
    const box = root.getBoundingClientRect();
    registry.set(OUTLINER, { x: box.left, y: box.top, width: box.width, height: box.height, held } satisfies Bounds);
  };

  // Rules 2, 3 and 5: what it shows.
  const render = () => {
    const outline = (registry.get(OUTLINE) as Outline | undefined) ?? [];
    const kinds = registry.get(KINDS) as Kinds | undefined;
    const lines = registry.get(LINES) as Lines | undefined;
    const status = registry.get(STATUS) as Status | undefined;
    const selected = registry.get(SELECTED) as string | null | undefined;
    const back = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>(".outline [data-focus]")?.dataset.focus;
    const kindChip = (kind: Kind) => {
      const on = kinds === undefined || kinds[kind];
      const b = button(`chip kind fam-${FAMILY[kind]}`, `${on ? "Hide" : "Show"} ${kindName(kind)}`, `kind:${kind}`, kindIcon(kind));
      b.setAttribute("aria-pressed", String(on));
      b.addEventListener("click", () => flipKind(registry, kind));
      return b;
    };
    root.replaceChildren();
    if (form === "shapes") {
      // Rule 5: the column.
      const strip = document.createElement("div");
      strip.className = "strip";
      const open = button("icon", "Open the outline", "btn:open", glyph("unfold"));
      open.addEventListener("click", () => setForm("open"));
      const rule = document.createElement("div");
      rule.className = "rule";
      strip.append(open, rule, ...ORDER.map(kindChip));
      root.append(strip);
    } else {
      // Rule 2: the header.
      const head = document.createElement("header");
      head.className = "head";
      const title = document.createElement("span");
      title.className = "title";
      title.textContent = "Outline";
      const count = document.createElement("span");
      count.className = "count";
      count.textContent = String(outline.length);
      const arrange = button("icon", "Arrange", "btn:arrange", glyph("arrange"));
      arrange.addEventListener("click", () => game.events.emit(ARRANGE));
      head.append(title, count, arrange);
      if (form === "open") {
        const fold = button("icon", "Fold", "btn:fold", glyph("fold"));
        fold.addEventListener("click", () => setForm(docked ? "shapes" : "header"));
        head.append(fold);
      } else {
        const unfold = button("icon", "Unfold", "btn:unfold", glyph("unfold"));
        unfold.addEventListener("click", () => setForm("open"));
        const shapes = button("icon", "Fold to shapes", "btn:shapes", glyph("shapes"));
        shapes.addEventListener("click", () => setForm("shapes"));
        head.append(unfold, shapes);
      }
      const dock = button("icon", docked ? "Float" : "Dock", "btn:dock", glyph(docked ? "float" : "dock"));
      dock.addEventListener("click", () => {
        docked = !docked;
        if (form === "header") form = "open";
        render();
      });
      head.append(dock);
      root.append(head);
      if (form === "open") {
        const said = words(status);
        if (said !== null) {
          const p = document.createElement("p");
          p.className = "said";
          p.textContent = said;
          root.append(p);
        }
        // Rule 2: what to show.
        const show = document.createElement("div");
        show.className = "show";
        const types = document.createElement("div");
        types.className = "chips";
        for (const type of TYPES) {
          const on = lines === undefined || lines[type];
          const n = status?.kind === "drawn" ? String(status.counts[type]) : "";
          const b = button("chip line", `${on ? "Hide" : "Show"} ${NAMES[type]}`, `line:${type}`, `${lineIcon(type)}<span class="n">${n}</span>`);
          b.setAttribute("aria-pressed", String(on));
          b.addEventListener("click", () => flipLine(registry, type));
          types.append(b);
        }
        show.append(types);
        for (const half of [ORDER.slice(0, 4), ORDER.slice(4)]) {
          const row = document.createElement("div");
          row.className = "chips";
          row.append(...half.map(kindChip));
          show.append(row);
        }
        const rule = document.createElement("div");
        rule.className = "rule";
        root.append(show, rule);
        // Rules 2 and 3: the list, by family.
        const list = document.createElement("div");
        list.className = "list";
        for (const [family, members] of FAMILIES) {
          const entries = outline
            .filter((e) => members.includes(e.kind) && (kinds === undefined || kinds[e.kind]))
            .sort((a, b) => members.indexOf(a.kind) - members.indexOf(b.kind) || a.label.localeCompare(b.label));
          const shut = folded.has(family);
          const group = button("group", `${shut ? "Unfold" : "Fold"} ${family}`, `group:${family}`, `${glyph("chevron")}<span class="label">${family}</span><span class="n">${entries.length}</span>`);
          group.setAttribute("aria-expanded", String(!shut));
          group.addEventListener("click", () => {
            if (shut) folded.delete(family);
            else folded.add(family);
            render();
          });
          list.append(group);
          if (shut) continue;
          for (const entry of entries) {
            const row = button(`row fam-${FAMILY[entry.kind]}${entry.bright ? "" : " faint"}${entry.key === selected ? " open" : ""}`, entry.label, `row:${entry.key}`, `${kindIcon(entry.kind)}<span class="name"></span>`);
            (row.querySelector(".name") as HTMLElement).textContent = entry.label;
            row.removeAttribute("aria-label");
            if (entry.key === selected) row.setAttribute("aria-current", "true");
            // Rule 4.
            row.addEventListener("click", () => game.events.emit(FOCUS, entry.key));
            row.addEventListener("mouseenter", () => game.events.emit(HOVER, entry.key));
            row.addEventListener("mouseleave", () => game.events.emit(HOVER, null));
            row.addEventListener("focus", () => game.events.emit(HOVER, entry.key));
            row.addEventListener("blur", () => game.events.emit(HOVER, null));
            list.append(row);
          }
        }
        // Rule 4: the arrows move along the list.
        list.addEventListener("keydown", (event) => {
          const all = [...list.querySelectorAll<HTMLElement>("button")];
          const at = all.indexOf(document.activeElement as HTMLElement);
          const to = event.key === "ArrowDown" ? at + 1 : event.key === "ArrowUp" ? at - 1 : event.key === "Home" ? 0 : event.key === "End" ? all.length - 1 : null;
          if (to === null || at === -1) return;
          event.preventDefault();
          all[Math.max(0, Math.min(all.length - 1, to))]?.focus();
        });
        root.append(list);
      }
    }
    if (back) root.querySelector<HTMLElement>(`[data-focus="${CSS.escape(back)}"]`)?.focus();
    place();
  };
  // A card against the window's right edge keeps that edge as it folds and unfolds.
  const setForm = (next: Form) => {
    const before = width();
    const right = left === null || window.innerWidth - (left + before) <= SNAP;
    form = next;
    if (!docked && right) left = window.innerWidth - width() - EDGE;
    render();
  };

  // Rule 6: dragging by the header, or by the folded column.
  let grab: { x: number; y: number; from: { x: number; y: number }; moved: boolean } | null = null;
  root.addEventListener("pointerdown", (event) => {
    const target = event.target as Element;
    if (event.button !== 0 || target.closest("button") || !target.closest(".head, .strip")) return;
    const box = root.getBoundingClientRect();
    grab = { x: event.clientX - box.left, y: event.clientY - box.top, from: { x: event.clientX, y: event.clientY }, moved: false };
    root.setPointerCapture(event.pointerId);
  });
  root.addEventListener("pointermove", (event) => {
    if (!grab) return;
    if (!grab.moved && Math.hypot(event.clientX - grab.from.x, event.clientY - grab.from.y) < CLICK) return;
    grab.moved = true;
    held = true;
    if (docked) {
      docked = false;
      render();
    }
    left = event.clientX - grab.x;
    top = event.clientY - grab.y;
    zone.classList.toggle("on", event.clientX < DOCK);
    zone.style.top = `${TABS + EDGE}px`;
    place();
  });
  const drop = (event: PointerEvent) => {
    if (!grab) return;
    const moved = grab.moved;
    grab = null;
    if (root.hasPointerCapture(event.pointerId)) root.releasePointerCapture(event.pointerId);
    if (!moved) return;
    held = false;
    if (zone.classList.contains("on")) {
      docked = true;
      if (form === "header") form = "open";
    } else if (left !== null) {
      const w = width();
      const floor = (sheetTop() ?? window.innerHeight) - EDGE;
      if (left < SNAP) left = EDGE;
      if (window.innerWidth - (left + w) < SNAP) left = window.innerWidth - w - EDGE;
      if (top - TABS < SNAP) top = TABS + EDGE;
      if (floor - (top + root.offsetHeight) < SNAP - EDGE) top = Math.max(TABS + EDGE, floor - root.offsetHeight);
    }
    zone.classList.remove("on");
    render();
  };
  root.addEventListener("pointerup", drop);
  root.addEventListener("pointercancel", drop);

  // Rule 7: the sheet coming, going and being resized moves the floor.
  let queued = false;
  const later = () => {
    if (queued) return;
    queued = true;
    requestAnimationFrame(() => {
      queued = false;
      place();
    });
  };
  const sheets = new MutationObserver(later);
  const watched = new WeakSet<Element>();
  const watch = () => {
    for (const sheet of document.querySelectorAll(".sheet")) {
      if (watched.has(sheet)) continue;
      watched.add(sheet);
      sheets.observe(sheet, { attributes: true, attributeFilter: ["hidden", "style", "class"] });
    }
    later();
  };
  new MutationObserver(watch).observe(document.body, { childList: true });
  window.addEventListener("resize", later);
  // A key set for the first time is `setdata`; every change after, `changedata-<key>`.
  const keys = [OUTLINE, SELECTED, KINDS, LINES, STATUS];
  for (const key of keys) registry.events.on(`changedata-${key}`, render);
  registry.events.on("setdata", (_parent: unknown, key: string) => keys.includes(key) && render());
  watch();
  render();
}
