/**
 * The scaffold's browser SCRIPT — a classic bundle injected into a
 * page this plugin does NOT own, declared in objectiveai.json as
 * `scripts[].module` and named when a tab spawns a browser
 * (`openViewerTab({ url, key, script })`).
 *
 * Its ENTIRE capability surface is `__objectiveai` — a closure-local
 * binding the viewer's injection wrapper provides (the page can never
 * reach it): the child-side mailbox toward the SPAWNING tab, and
 * nothing else.
 *
 *   __objectiveai.send(payload)        -> boolean
 *   __objectiveai.subscribe(timeout?)  -> Promise<unknown[]>
 *   __objectiveai.list(pending?)       -> Promise<unknown[]>
 *
 * No Tauri IPC, no SDK, no imports at runtime: the page shares this
 * JS world, so anything more powerful here would be hijackable. Keep
 * scripts dumb — collect, display, send; let the trusted parent tab
 * decide what the data means.
 *
 * The shadow root keeps the page's styles off these nodes and these
 * rules off the page's. It does NOT stop the page removing the host
 * element — real overlays re-assert themselves from a
 * MutationObserver.
 */

declare const __objectiveai: {
  send(payload: unknown): boolean;
  subscribe(timeoutMs?: number): Promise<unknown[]>;
  list(pending?: boolean): Promise<unknown[]>;
};

import css from "./overlay.css";

const sheet = new CSSStyleSheet();
sheet.replaceSync(css);

const host = document.createElement("div");
const shadow = host.attachShadow({ mode: "closed" });
shadow.adoptedStyleSheets = [sheet];

const panel = document.createElement("div");
panel.className = "panel";

const label = document.createElement("div");
label.className = "label";
label.textContent = "ObjectiveAI: a credential was requested for this site";

const input = document.createElement("input");
input.type = "password";
input.placeholder = "credential";

const button = document.createElement("button");
button.textContent = "send to viewer";
button.addEventListener("click", () => {
  if (input.value.length === 0) {
    return;
  }
  // Child→parent mailbox: the spawning tab drains this with
  // `subscribeViewerTab(key)` and treats it as UNTRUSTED input.
  const delivered = __objectiveai.send({ credential: input.value });
  label.textContent = delivered
    ? "sent — confirm in the viewer tab"
    : "bridge unavailable";
});

panel.append(label, input, button);
shadow.append(panel);
document.documentElement.append(host);
