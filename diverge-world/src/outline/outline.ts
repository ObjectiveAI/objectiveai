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

/** The card's and the rail's width (the viewer's rail), the widest a card is resized to, and the shapes' column's. */
const WIDTH = 244;
const WIDEST = 420;
const COLUMN = 46;

/** The shortest a resized card stands, in pixels. */
const SHORTEST = 160;

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

/** The form the outline takes: open; folded to its header; folded to the kinds' shapes; folded to one button. */
type Form = "open" | "header" | "shapes" | "button";

/** The window's edge a floating card was last let go nearest. */
type Edge = "left" | "right" | "top" | "bottom";

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
 * 5. Folded, it is its header alone; or the kinds' shapes, each still a
 *    chip that shows or hides its kind, upright at a side edge and lying
 *    along the top or bottom edge where it was let go nearest those; or
 *    one button, Outline and how many nodes, which opens it again where
 *    it stands. The first button of each opens it again.
 * 6. Dragged by its header it floats anywhere in the window under the
 *    strip; let go near an edge, it snaps `EDGE` in; let go near the
 *    window's left (`DOCK`), where a dashed outline shows while it is
 *    held there, it docks as a rail the window's height, `WIDTH` wide —
 *    or `COLUMN` folded — and the sheet opens beside it. Dragged off the
 *    rail, or its float button pressed, it floats again; its dock button
 *    docks it.
 * 7. A floating card stops `EDGE` above the sheet when one is open, or
 *    the window's foot, and its list scrolls within what is left. The
 *    grip at an open card's lower corner resizes it, `WIDTH` to `WIDEST`
 *    wide and no shorter than `SHORTEST`: the right corner, or the left
 *    one on a card against the window's right edge, which grows leftward.
 *    It keeps that size until the page closes.
 * 8. It keeps no files: where it stands and how it is folded live for
 *    the page alone.
 * 9. It is drawn again only when something it shows has changed — the
 *    nodes, their names and brightness, the open one, what is shown,
 *    the counts, its form and folds — and its list keeps where it was
 *    scrolled, so a busy daemon neither scrolls it back nor drops the
 *    row under the mouse.
 */
export function outline(game: Phaser.Game): void {
  const registry = game.registry;
  let form: Form = "open";
  let docked = false;
  let left: number | null = null;
  let top = TABS + EDGE;
  let held = false;
  let edge: Edge = "right";
  let size: { w: number; h: number } | null = null;
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
  /** The card's width in its form, or `null` where its content sets it: the shapes lying along an edge, the one button. */
  const width = (): number | null => {
    if (form === "shapes") return docked || edge === "left" || edge === "right" ? COLUMN : null;
    if (form === "button") return null;
    return form === "open" && !docked && size ? size.w : WIDTH;
  };

  // Rules 6 and 7: where it stands, and how tall it may be.
  const place = () => {
    const w = width();
    root.dataset.form = form;
    root.dataset.docked = String(docked);
    root.dataset.along = String(form === "shapes" && w === null);
    root.classList.toggle("held", held);
    document.documentElement.style.setProperty("--outline-left", docked ? `${w ?? COLUMN}px` : "0px");
    if (docked) {
      Object.assign(root.style, { left: "0px", top: `${TABS}px`, width: `${w ?? COLUMN}px`, height: `${window.innerHeight - TABS}px`, maxHeight: "" });
    } else {
      const floor = (sheetTop() ?? window.innerHeight) - EDGE;
      root.style.width = w === null ? "" : `${w}px`;
      const across = root.offsetWidth;
      left = Math.max(0, Math.min(window.innerWidth - across, left ?? window.innerWidth - across - EDGE));
      // An open card may run under the floor and scroll, so 40px of it is enough; a folded one never scrolls, so all of it stays above.
      top = Math.max(TABS, Math.min(Math.max(TABS, floor - (form === "open" ? 40 : root.scrollHeight)), top));
      const most = Math.max(40, floor - top);
      root.dataset.anchor = window.innerWidth - (left + across) <= SNAP ? "right" : "left";
      Object.assign(root.style, { left: `${left}px`, top: `${top}px`, height: form === "open" && size ? `${Math.min(size.h, most)}px` : "", maxHeight: `${most}px` });
    }
    const box = root.getBoundingClientRect();
    registry.set(OUTLINER, { x: box.left, y: box.top, width: box.width, height: box.height, held } satisfies Bounds);
  };

  // Rule 9: what was last drawn, so a world that changes nothing it shows draws nothing.
  let drawn = "";
  // Rules 2, 3 and 5: what it shows.
  const render = () => {
    const outline = (registry.get(OUTLINE) as Outline | undefined) ?? [];
    const kinds = registry.get(KINDS) as Kinds | undefined;
    const lines = registry.get(LINES) as Lines | undefined;
    const status = registry.get(STATUS) as Status | undefined;
    const selected = registry.get(SELECTED) as string | null | undefined;
    const said = words(status);
    // Rule 9.
    const counts = status?.kind === "drawn" ? TYPES.map((type) => status.counts[type]) : null;
    const showing = JSON.stringify([form, docked, [...folded].sort(), selected ?? null, kinds ?? null, lines ?? null, said, counts, outline.map((e) => [e.key, e.kind, e.label, e.bright])]);
    if (showing === drawn) {
      place();
      return;
    }
    drawn = showing;
    const scrolled = root.querySelector<HTMLElement>(".list")?.scrollTop ?? 0;
    const back = (document.activeElement as HTMLElement | null)?.closest<HTMLElement>(".outline [data-focus]")?.dataset.focus;
    const kindChip = (kind: Kind) => {
      const on = kinds === undefined || kinds[kind];
      const b = button(`chip kind fam-${FAMILY[kind]}`, `${on ? "Hide" : "Show"} ${kindName(kind)}`, `kind:${kind}`, kindIcon(kind));
      b.setAttribute("aria-pressed", String(on));
      b.addEventListener("click", () => flipKind(registry, kind));
      return b;
    };
    root.replaceChildren();
    if (form === "button") {
      // Rule 5: one button; a drag moves it, a press opens the outline where it stands.
      const pill = button("pill", "Open the outline", "btn:pill", `${glyph("list")}<span class="title">Outline</span><span class="n">${outline.length}</span>`);
      pill.addEventListener("click", () => {
        if (dragged) dragged = false;
        else setForm("open");
      });
      root.append(pill);
    } else if (form === "shapes") {
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
      head.append(title, count);
      if (form === "open") {
        const arrange = button("icon", "Arrange", "btn:arrange", glyph("arrange"));
        arrange.addEventListener("click", () => game.events.emit(ARRANGE));
        const fold = button("icon", "Fold", "btn:fold", glyph("fold"));
        fold.addEventListener("click", () => setForm(docked ? "shapes" : "header"));
        head.append(arrange, fold);
      } else {
        const unfold = button("icon", "Unfold", "btn:unfold", glyph("unfold"));
        unfold.addEventListener("click", () => setForm("open"));
        const shapes = button("icon", "Fold to shapes", "btn:shapes", glyph("shapes"));
        shapes.addEventListener("click", () => setForm("shapes"));
        const one = button("icon", "Fold to a button", "btn:one", glyph("one"));
        one.addEventListener("click", () => setForm("button"));
        head.append(unfold, shapes, one);
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
        list.scrollTop = scrolled;
        // Rule 7: the corner's grip, on a floating card.
        if (!docked) {
          const grip = document.createElement("div");
          grip.className = "grip";
          grip.title = "Resize";
          grip.innerHTML = glyph("resize");
          root.append(grip);
        }
      }
    }
    if (back) root.querySelector<HTMLElement>(`[data-focus="${CSS.escape(back)}"]`)?.focus();
    place();
  };
  // A card against the window's right edge keeps that edge as it folds and unfolds.
  // …and a card against the foot keeps the foot, opening upward.
  const setForm = (next: Form) => {
    const floor = () => (sheetTop() ?? window.innerHeight) - EDGE;
    const right = left === null || window.innerWidth - (left + root.offsetWidth) <= SNAP;
    const foot = floor() - (top + root.offsetHeight) <= SNAP;
    form = next;
    render();
    if (docked) return;
    if (right) left = window.innerWidth - root.offsetWidth - EDGE;
    if (foot) top = Math.max(TABS + EDGE, floor() - root.scrollHeight);
    place();
  };

  // Rule 6: dragging by the header, the folded column or the one button; rule 7: resizing by the grip. The pointer is
  // followed on the window, not captured, so a press on the one button that never moves stays its own click.
  let grab: { x: number; y: number; from: { x: number; y: number }; moved: boolean } | null = null;
  let stretch: { x: number; y: number; w: number; h: number; right: number | null } | null = null;
  let dragged = false;
  const move = (event: PointerEvent) => {
    if (stretch) {
      const floor = (sheetTop() ?? window.innerHeight) - EDGE;
      // A card against the right edge grows leftward, its right edge holding still.
      const across = stretch.right === null ? event.clientX - stretch.x : stretch.x - event.clientX;
      size = {
        w: Math.max(WIDTH, Math.min(WIDEST, stretch.w + across)),
        h: Math.max(SHORTEST, Math.min(floor - top, stretch.h + event.clientY - stretch.y)),
      };
      if (stretch.right !== null) left = stretch.right - size.w;
      place();
      return;
    }
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
    zone.classList.toggle("on", event.clientX < DOCK && form !== "button");
    zone.style.top = `${TABS + EDGE}px`;
    place();
  };
  const drop = () => {
    removeEventListener("pointermove", move);
    removeEventListener("pointerup", drop);
    removeEventListener("pointercancel", drop);
    if (stretch) {
      stretch = null;
      held = false;
      place();
      return;
    }
    if (!grab) return;
    const moved = grab.moved;
    grab = null;
    if (!moved) return;
    dragged = form === "button";
    held = false;
    if (zone.classList.contains("on")) {
      docked = true;
      if (form === "header") form = "open";
    } else if (left !== null) {
      // Rule 5: the edge it was let go nearest sets which way the shapes lie; rule 6: near an edge, it snaps in.
      const floor = (sheetTop() ?? window.innerHeight) - EDGE;
      const gaps: Record<Edge, number> = { left, right: window.innerWidth - (left + root.offsetWidth), top: top - TABS, bottom: floor - (top + root.offsetHeight) };
      edge = (Object.keys(gaps) as Edge[]).sort((a, b) => gaps[a] - gaps[b])[0];
      // Turned to lie along or stand at the edge first; then snapped by the gaps as they were when it was let go, so a
      // strip that turns upright against the right edge still lands against it.
      place();
      if (gaps.left < SNAP) left = EDGE;
      if (gaps.right < SNAP) left = window.innerWidth - root.offsetWidth - EDGE;
      if (gaps.top < SNAP) top = TABS + EDGE;
      if (gaps.bottom < SNAP) top = Math.max(TABS + EDGE, floor - root.offsetHeight);
    }
    zone.classList.remove("on");
    render();
    place();
  };
  root.addEventListener("pointerdown", (event) => {
    const target = event.target as Element;
    if (event.button !== 0) return;
    const box = root.getBoundingClientRect();
    if (target.closest(".grip")) {
      stretch = { x: event.clientX, y: event.clientY, w: box.width, h: box.height, right: root.dataset.anchor === "right" ? box.right : null };
      held = true;
      event.preventDefault();
    } else {
      const pill = target.closest(".pill");
      if ((target.closest("button") && !pill) || !target.closest(".head, .strip, .pill")) return;
      grab = { x: event.clientX - box.left, y: event.clientY - box.top, from: { x: event.clientX, y: event.clientY }, moved: false };
    }
    addEventListener("pointermove", move);
    addEventListener("pointerup", drop);
    addEventListener("pointercancel", drop);
  });

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
