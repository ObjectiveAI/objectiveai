// The nodes alight: who is working, whose container is down, which
// account is away.

import type * as Phaser from "phaser";
import type { Live } from "../world/model";
import { type Node, type Nodes, setBright } from "./nodes";
import { ink } from "./palette";
import { type Offset, OUTLINE } from "./textures";

/** One round of the trace: drawn on in the first half, let go in the second, in milliseconds. */
const PERIOD = 2400;

/** The trace's width, in pixels, and how much of the outline it holds still when the system asks for less motion. */
const WIDTH = 2.5;
const STILL = 0.5;

/** How faint the trace of a dimmed node is drawn. */
const DIMMED = 0.25;

/** The live state, told each node's by its key. */
export type Lit = { set(key: string, live: Live | null, deployed?: boolean): void; forget(key: string): void };

/** Ease in and out along a sine, `x` from 0 to 1. */
const ease = (x: number) => (1 - Math.cos(Math.PI * x)) / 2;

/** Stroke the part of `path` from `from` to `to` (shares of its length) about `x`, `y`, in the graphics' line style. */
function along(g: Phaser.GameObjects.Graphics, path: Offset[], x: number, y: number, from: number, to: number): void {
  const lengths = [0];
  for (let i = 1; i < path.length; i++) lengths.push(lengths[i - 1] + Math.hypot(path[i].x - path[i - 1].x, path[i].y - path[i - 1].y));
  const total = lengths[lengths.length - 1];
  const a = from * total;
  const b = to * total;
  if (b - a <= 0.01) return;
  const at = (d: number) => {
    let i = 1;
    while (i < lengths.length - 1 && lengths[i] < d) i++;
    const span = lengths[i] - lengths[i - 1] || 1;
    const s = (d - lengths[i - 1]) / span;
    return { x: x + path[i - 1].x + (path[i].x - path[i - 1].x) * s, y: y + path[i - 1].y + (path[i].y - path[i - 1].y) * s };
  };
  g.beginPath();
  const start = at(a);
  g.moveTo(start.x, start.y);
  for (let i = 1; i < lengths.length; i++) if (lengths[i] > a && lengths[i] < b) g.lineTo(x + path[i].x, y + path[i].y);
  const end = at(b);
  g.lineTo(end.x, end.y);
  g.strokePath();
}

/**
 * Light the nodes.
 *
 * The rules:
 *
 * 1. An agent or a tool `active` — its loop running, a call or a read in
 *    flight — is traced: a line in ink draws itself round the node's own
 *    outline, then lets go from where it began, once every `PERIOD`. No
 *    glow, no dot: the shape itself is what moves.
 * 2. When the system asks for less motion the trace stands still,
 *    `STILL` of the outline drawn, so working is still a shape that
 *    differs.
 * 3. A node not bright is faint (`setBright`): an agent or a tool whose
 *    container is not deployed, an account not connected, a volume
 *    mounted into no deployed container. Faint is a still shape in the
 *    strong edge, not a fade.
 * 4. The trace follows its node as it is dragged or glides, hides with
 *    it, and dims with it while another node is focused.
 */
export function makeLit(scene: Phaser.Scene, nodes: Nodes): Lit {
  const working = new Set<string>();
  const g = scene.add.graphics().setDepth(5.5);
  const less = matchMedia("(prefers-reduced-motion: reduce)");
  const draw = () => {
    g.clear();
    if (working.size === 0) return;
    // Rules 1 and 2.
    const t = less.matches ? null : (scene.time.now % PERIOD) / PERIOD;
    const [from, to] = t === null ? [0, STILL] : t < 0.5 ? [0, ease(t * 2)] : [ease((t - 0.5) * 2), 1];
    for (const key of working) {
      const node: Node | undefined = nodes.byKey.get(key);
      // Rule 4.
      if (!node || !node.image.visible) continue;
      g.lineStyle(WIDTH, ink(), node.image.tintTopLeft === 0xffffff ? 1 : DIMMED);
      along(g, OUTLINE[node.entity.kind], node.image.x, node.image.y, from, to);
    }
  };
  scene.events.on("update", draw);
  scene.events.once("shutdown", () => {
    scene.events.off("update", draw);
    g.destroy();
  });
  return {
    // A node removed: its trace forgotten.
    forget(key) {
      working.delete(key);
    },
    set(key, live, deployed) {
      const node = nodes.byKey.get(key);
      if (!node) return;
      // Rule 1.
      if (live?.on && live.kind === "active") working.add(key);
      else working.delete(key);
      // Rule 3.
      if (deployed !== undefined) setBright(node, deployed);
    },
  };
}
