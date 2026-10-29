// One piece of content — text, an image, a sound, a resource — as the
// sheet shows it.

import { asJson, jsonBlock } from "./json";
import { renderMarkdown } from "./markdown";

/** A text block: rendered markdown, or JSON when the text is a JSON object or array. */
export function textElement(text: string): HTMLElement {
  const json = asJson(text);
  if (json !== undefined) return jsonBlock(json);
  const el = document.createElement("div");
  el.className = "text";
  renderMarkdown(el, text);
  return el;
}

/** Base64 data of a type, as an image. */
export function imageElement(data: string, mimeType: string): HTMLElement {
  const img = document.createElement("img");
  img.className = "media";
  img.loading = "lazy";
  img.alt = "image";
  img.src = `data:${mimeType};base64,${data}`;
  return img;
}

/** Base64 data of a type, as a sound with its controls. */
export function audioElement(data: string, mimeType: string): HTMLElement {
  const audio = document.createElement("audio");
  audio.controls = true;
  audio.preload = "metadata";
  audio.src = `data:${mimeType};base64,${data}`;
  return audio;
}

/** A resource or a link to one: its name or URI, set as code. */
export function resourceElement(label: string): HTMLElement {
  const el = document.createElement("div");
  el.className = "resource";
  el.textContent = label;
  return el;
}

/**
 * One MCP content block, as a tool's response carries it.
 *
 * The rules:
 *
 * 1. `text` is a text block (`textElement`).
 * 2. `image` and `audio` are drawn from their base64 data.
 * 3. `resource_link` is its name and URI; `resource` is its text when
 *    it has some, else its URI.
 * 4. Anything else is shown as JSON, so nothing is mislabelled.
 */
export function contentElement(block: { type: string } & Record<string, unknown>): HTMLElement {
  const str = (key: string) => (typeof block[key] === "string" ? (block[key] as string) : "");
  switch (block.type) {
    case "text":
      return textElement(str("text"));
    case "image":
      return imageElement(str("data"), str("mimeType"));
    case "audio":
      return audioElement(str("data"), str("mimeType"));
    case "resource_link":
      return resourceElement(`${str("name")} — ${str("uri")}`);
    case "resource": {
      const r = (block.resource ?? {}) as Record<string, unknown>;
      return typeof r.text === "string" ? textElement(r.text) : resourceElement(String(r.uri ?? "resource"));
    }
    default:
      return jsonBlock(block);
  }
}
