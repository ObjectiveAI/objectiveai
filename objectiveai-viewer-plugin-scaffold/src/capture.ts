/**
 * The scaffold's browser SCRIPT — a classic bundle injected into a
 * page this plugin does NOT own, declared in objectiveai.json as
 * `scripts[].module` and named when a tab spawns a browser
 * (`openViewerTab({ url, key, script })`).
 *
 * This one is written for the page the credential handler opens:
 * `https://pastebin.com/`, whose new-paste form has exactly ONE body
 * field. It paints a panel with ONE button that reads whatever is
 * typed there and sends it to the SPAWNING TAB, which validates it and
 * writes it back over the channel.
 *
 * A script's ENTIRE capability surface is `__objectiveai` — a
 * closure-local binding the viewer's injection wrapper provides (the
 * page can never reach it): the child-side mailbox toward the
 * spawning tab, and nothing else.
 *
 *   __objectiveai.send(payload)        -> boolean
 *   __objectiveai.subscribe(timeout?)  -> Promise<unknown[]>
 *   __objectiveai.list(pending?)       -> Promise<unknown[]>
 *
 * No Tauri IPC, no SDK, no imports at runtime: the page shares this
 * JS world, so anything more powerful here would be hijackable by the
 * page. Keep scripts dumb — read, display, send; let the trusted
 * parent tab decide what the data means and whether to act on it.
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

import css from "./capture.css";

/** The field this script harvests.
 *
 * Pastebin's new-paste page carries exactly one paste body, and it is
 * a TEXTAREA: `#postform-text`, `name="PostForm[text]"`. Naming it
 * outright is the point of choosing this page — "the first typable
 * field" is a guess on any page that has several, and guessing wrong
 * silently sends the wrong string.
 *
 * The fallback keeps the script useful pointed anywhere else. */
function field(): HTMLTextAreaElement | HTMLInputElement | null {
  const paste = document.querySelector<HTMLTextAreaElement>(
    'textarea#postform-text, textarea[name="PostForm[text]"]',
  );
  if (paste) return paste;
  return document.querySelector<HTMLTextAreaElement | HTMLInputElement>(
    'textarea, input[type="text"], input[type="password"], input[type="search"], input:not([type])',
  );
}

const sheet = new CSSStyleSheet();
sheet.replaceSync(css);

const host = document.createElement("div");
const shadow = host.attachShadow({ mode: "closed" });
shadow.adoptedStyleSheets = [sheet];

const panel = document.createElement("div");
panel.className = "panel";

const label = document.createElement("div");
label.className = "label";
label.textContent = "ObjectiveAI: type a value into the paste, then send it back";

const button = document.createElement("button");
button.textContent = "send to ObjectiveAI";
button.addEventListener("click", () => {
  const input = field();
  const value = input?.value ?? "";
  if (value.length === 0) {
    label.textContent = "nothing to send — the field is empty";
    return;
  }
  // Child→parent mailbox. The spawning tab drains this with
  // `subscribeViewerTab(key)` and treats it as UNTRUSTED input: it
  // validates the shape before doing anything with the value.
  const delivered = __objectiveai.send({ credential: value });
  label.textContent = delivered
    ? "sent — this tab closes when the viewer accepts it"
    : "bridge unavailable";
});

panel.append(label, button);
shadow.append(panel);
document.documentElement.append(host);
