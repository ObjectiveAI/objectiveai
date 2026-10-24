import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const APP = fileURLToPath(new URL("..", import.meta.url));
const read = (p: string) => readFileSync(join(APP, p), "utf8");
const conf = JSON.parse(read("src-tauri/tauri.conf.json"));
const capability = JSON.parse(read("src-tauri/capabilities/default.json"));

function code(dir: string): string[] {
  const out: string[] = [];
  for (const n of readdirSync(dir).sort()) {
    const p = join(dir, n);
    if (statSync(p).isDirectory()) out.push(...code(p));
    else if (/\.tsx?$/.test(n) && !/\.test\.tsx?$/.test(n)) out.push(p);
  }
  return out;
}

describe("what the window may load", () => {
  const csp: Record<string, string> = conf.app.security.csp;

  it("scripts and styles come only from the app itself; pictures also from data: and blob:", () => {
    expect(csp).toEqual({
      "default-src": "'self'",
      "script-src": "'self'",
      "style-src": "'self'",
      "img-src": "'self' data: blob:",
      // Tauri's own calls: ipc://localhost on macOS and Linux, http://ipc.localhost on Windows.
      "connect-src": "ipc: http://ipc.localhost",
      "object-src": "'none'",
      "base-uri": "'none'",
    });
    expect(Object.values(csp).join(" ")).not.toMatch(/unsafe-|\*/);
  });

  it("the page has nothing inline for the policy to block", () => {
    const html = read("index.html");
    expect(html).not.toMatch(/<script(?![^>]*\bsrc=)[^>]*>/);
    expect(html).not.toMatch(/<style\b|\sstyle=|\son[a-z]+=/i);
    const risky = code(join(APP, "src")).filter((f) => /dangerouslySetInnerHTML|\.innerHTML\s*=|\beval\(|new Function\(|<style\b/.test(readFileSync(f, "utf8")));
    expect(risky.map((f) => f.slice(APP.length))).toEqual([]);
  });
});

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
