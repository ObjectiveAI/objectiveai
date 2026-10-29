// An agent's text as markdown, made safe to show.

import { micromark } from "micromark";
import { gfm, gfmHtml } from "micromark-extension-gfm";

/** Text that has two markdown characters together, or a fence: worth parsing. Anything else is shown as it is. */
const MARKDOWN = /[#*`[\]|>~_-]{2}|```|^\s*([-*+]|\d+\.)\s/m;

/**
 * Fill `into` with `text`, rendered.
 *
 * The rules:
 *
 * 1. Text with no sign of markdown is set as plain text, unparsed.
 * 2. Anything else is markdown with GitHub's extensions. Raw HTML in it
 *    is never HTML: micromark escapes it unless told otherwise, and it
 *    is not told.
 * 3. A link is never a link: every `<a>` becomes a `<span class="link">`
 *    holding its text, so an agent's words cannot navigate the page.
 *    An image in markdown becomes its alt text, so nothing is fetched.
 * 4. Newlines are kept as the agent wrote them (`white-space:
 *    pre-line` in the sheet's style).
 */
export function renderMarkdown(into: HTMLElement, text: string): void {
  // Rule 1.
  if (!MARKDOWN.test(text)) {
    into.textContent = text;
    return;
  }
  // Rule 2.
  into.innerHTML = micromark(text, { extensions: [gfm()], htmlExtensions: [gfmHtml()] });
  // Rule 3.
  for (const a of into.querySelectorAll("a")) {
    const span = document.createElement("span");
    span.className = "link";
    span.textContent = a.textContent;
    a.replaceWith(span);
  }
  for (const img of into.querySelectorAll("img")) {
    const span = document.createElement("span");
    span.className = "link";
    span.textContent = img.alt || "image";
    img.replaceWith(span);
  }
}
