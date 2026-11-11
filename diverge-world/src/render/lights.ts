// The lights: a green halo and a dot on the plate for every live entity.

import * as Phaser from "phaser";
import { corners } from "../world/hex";
import type { WorldMap } from "../world/map";
import type { Live } from "../world/model";
import { pixel, SIZE } from "./scale";

/** The live green, the sheet's own. */
const GREEN = 0x4ade80;

/** How long one breath of a working entity's pulse takes, and how long a light takes to go out, in milliseconds. */
const BREATH = 1600;
const FADE = 500;

/** The halo's alpha: steady, and the two ends of a pulse. */
const STEADY = 0.45;
const LOW = 0.15;
const HIGH = 0.6;

/** The lights, told each entity's state by its key. */
export type Lights = { set(key: string, live: Live | null): void };

/**
 * The lights over the map.
 *
 * The rules:
 *
 * 1. An entity `active` — an agent's loop, a tool's call or read in
 *    flight — has a green halo just outside its tile that slowly
 *    pulses, and a green dot at its plate's start.
 * 2. An account `connected` has the same halo, steady, and the dot.
 * 3. Neither, nothing: the tile as it is. A light going out fades.
 * 4. The halo lies under the tile and over the land; the dot over the
 *    plate.
 */
export function makeLights(scene: Phaser.Scene, map: WorldMap, plates: Map<string, Phaser.GameObjects.Text>): Lights {
  const shape = corners(SIZE + 7);
  const lit = new Map<string, { halo: Phaser.GameObjects.Graphics; dot: Phaser.GameObjects.Arc; tween: Phaser.Tweens.Tween | null }>();
  const light = (key: string) => {
    const known = lit.get(key);
    if (known) return known;
    const at = map.at.get(key);
    const plate = plates.get(key);
    if (!at || !plate) return null;
    const c = pixel(at);
    // Rule 4.
    const halo = scene.add.graphics().setDepth(1.9).setAlpha(0);
    halo.fillStyle(GREEN, 1);
    halo.fillPoints(shape.map((p) => new Phaser.Math.Vector2(c.x + p.x, c.y + p.y)), true);
    const left = plate.getLeftCenter();
    const dot = scene.add.circle((left.x ?? c.x) - 5, left.y ?? c.y, 3, GREEN).setDepth(3.1).setAlpha(0);
    const made = { halo, dot, tween: null as Phaser.Tweens.Tween | null };
    lit.set(key, made);
    return made;
  };
  return {
    set(key, live) {
      const l = light(key);
      if (!l) return;
      l.tween?.stop();
      l.tween = null;
      scene.tweens.killTweensOf([l.halo, l.dot]);
      if (live?.on) {
        l.dot.setAlpha(1);
        if (live.kind === "active") {
          // Rule 1.
          l.halo.setAlpha(LOW);
          l.tween = scene.tweens.add({ targets: l.halo, alpha: HIGH, duration: BREATH / 2, yoyo: true, repeat: -1, ease: "Sine.easeInOut" });
        } else {
          // Rule 2.
          l.halo.setAlpha(STEADY);
        }
        return;
      }
      // Rule 3.
      scene.tweens.add({ targets: [l.halo, l.dot], alpha: 0, duration: FADE });
    },
  };
}
