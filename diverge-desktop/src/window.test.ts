import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const APP = fileURLToPath(new URL("..", import.meta.url));
const read = (p: string) => readFileSync(join(APP, p), "utf8");
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
const capability = JSON.parse(read("src-tauri/capabilities/default.json"));

describe("zoom keys", () => {
  it("the window zooms with the keyboard (⌘ or Ctrl with + − 0), and may set its own zoom", () => {
    expect(conf.app.windows.find((w: { label: string }) => w.label === "main").zoomHotkeysEnabled).toBe(true);
    expect(capability.windows).toContain("main");
    expect(capability.permissions).toContain("core:webview:allow-set-webview-zoom");
  });

  it("the menu leaves those keys to the zoom", () => {
    const keys = [...read("src-tauri/src/main.rs").matchAll(/accelerator\("([^"]+)"\)/g)].map((m) => m[1]);
    expect(keys.length).toBeGreaterThan(0);
    expect(keys.filter((k) => /\+(=|-|0|Plus|Minus|Equal)$/i.test(k))).toEqual([]);
  });
});
