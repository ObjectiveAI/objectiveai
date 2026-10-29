// An agent's log, folded into messages as its items arrive.

import type { daemon } from "@diverge/sdk";
import { agoElement } from "./ago";
import { jsonBlock } from "./json";
import { audioElement, contentElement, imageElement, resourceElement, textElement } from "./media";
import { sender, toolName } from "./sender";

type Item = daemon.endpoints.agents.logs.server.response.ItemWrapper;
type Creator = daemon.creator.Creator;

/** What a run of text is: the agent's words, its reasoning, or its refusal. */
type TextKind = "text" | "reasoning" | "refusal";

/** A block whose content grows: its buffer, and how it is drawn again. */
type Growing = { buffer: string; draw: () => void };

/** A tool call's card: its arguments as they grow, its status mark, and where its response goes. */
type Call = { args: Growing; card: HTMLElement; status: HTMLElement; response: HTMLElement };

/** The message being added to: its role, as a key, and where its blocks go. */
type Open = { role: string; blocks: HTMLElement };

/**
 * The log of one agent, drawn into `body` as messages.
 *
 * The rules:
 *
 * 1. A message is a run of items with one role and one label. Each
 *    item continues the open message or starts a new one, and the
 *    label — with when the message began — is drawn once, at its head.
 * 2. Every assistant chunk joins the open assistant message, or starts
 *    one, labelled "assistant". Within it:
 *    - text fragments are added to the current text block, so any
 *      number in a row read as one text;
 *    - an image or a sound ends the current text block and is drawn
 *      below it, and text after it starts a new block below that;
 *    - reasoning and refusals are text blocks of their own kinds:
 *      reasoning a closed disclosure, a refusal drawn red;
 *    - a tool call is one card per call id, closed until opened: its
 *      tool name (`toolName`) and a mark — pending, done, or failed —
 *      over its arguments, the fragments added together and shown as
 *      JSON once they parse, and its response beneath them.
 * 3. A tool response goes into its call's card, matched by id, its
 *    content drawn block by block, the card marked failed when it is
 *    an error; the open message stays open. A response whose call the
 *    log has not shown is a message of its own, labelled "tool".
 * 4. A user part joins the open message only when that message is a
 *    user message from the same sender; its label is the sender
 *    (`sender`), never "user". Within it, text and media follow rule
 *    2's text rules, and the texts of two different messages are
 *    kept a blank line apart.
 * 5. A run's start and end close the open message and draw nothing;
 *    they only light and dim the sheet's live dot. An error is a red message of its own showing its JSON; a
 *    notification a message of its own. Usage draws nothing.
 * 6. A chunk made inside a tool call is drawn as if it were not.
 * 7. Growing blocks are drawn again at most once an animation frame,
 *    only those that grew; `grew` is called after every drawing.
 */
export class Fold {
  private open: Open | null = null;
  private text: { kind: TextKind; block: Growing; key: string | null } | null = null;
  private readonly calls = new Map<string, Call>();
  private readonly names = new Map<string, string>();
  private readonly dirty = new Set<Growing>();
  private frame: number | null = null;

  constructor(
    private readonly body: HTMLElement,
    private readonly images: Map<string, string>,
    private readonly grew: () => void,
    private readonly active: (on: boolean) => void,
  ) {}

  /** One item of the log, in order. */
  push(item: Item): void {
    // A user part is the one item with a sender and an inbox key; a tool response's loose members are never one.
    if ("sender" in item && "key" in item && item.type.startsWith("user_")) return this.user(item, item.sender as Creator);
    switch (item.type) {
      case "assistant_text_content":
      case "assistant_reasoning":
      case "assistant_refusal": {
        this.ensure("assistant", "assistant", "agent", item.created);
        const kind: TextKind = item.type === "assistant_text_content" ? "text" : item.type === "assistant_reasoning" ? "reasoning" : "refusal";
        this.appendText(kind, item.text, null);
        break;
      }
      case "assistant_image_content":
        this.ensure("assistant", "assistant", "agent", item.created);
        this.place(imageElement(item.data, item.mimeType));
        break;
      case "assistant_audio_content":
        this.ensure("assistant", "assistant", "agent", item.created);
        this.place(audioElement(item.data, item.mimeType));
        break;
      case "assistant_tool_call":
        this.ensure("assistant", "assistant", "agent", item.created);
        this.call(item.id, item.name, item.arguments ?? "");
        break;
      case "tool_response": {
        // Rule 3.
        const content = (item.content ?? []) as ({ type: string } & Record<string, unknown>)[];
        const drawn = content.map((block) => contentElement(block));
        if (content.length === 0 && item.structuredContent !== undefined) drawn.push(jsonBlock(item.structuredContent));
        const call = this.calls.get(item.id);
        if (call) {
          call.response.replaceChildren(...drawn);
          call.response.hidden = false;
          call.status.textContent = item.isError ? "✕" : "✓";
          call.card.classList.toggle("failed", item.isError === true);
          this.grew();
          break;
        }
        this.close();
        this.ensure("tool", "tool", "tool", item.created);
        for (const el of drawn) this.place(el);
        if (item.isError) this.open?.blocks.parentElement?.classList.add("failed");
        this.close();
        break;
      }
      case "active":
      case "inactive":
        // Rule 5.
        this.close();
        this.active(item.type === "active");
        break;
      case "error":
        this.close();
        this.ensure("error", "error", "error", item.created);
        this.place(jsonBlock(item.error));
        this.open?.blocks.parentElement?.classList.add("failed");
        this.close();
        break;
      case "notification":
        this.close();
        this.ensure("notification", item.is_fatal ? "fatal notification" : "notification", "tool", item.created);
        this.place(jsonBlock(item.message));
        this.close();
        break;
      case "usage":
        break;
      default:
        // Rule 4's parts never come without a sender; anything else is shown as it is.
        this.close();
        this.ensure("other", item.type, "tool", item.created);
        this.place(jsonBlock(item));
        this.close();
    }
  }

  /** A line of the sheet's own, outside any message: the log ended, was refused, or failed. */
  note(text: string): void {
    this.close();
    const el = document.createElement("div");
    el.className = "note";
    el.textContent = text;
    this.body.append(el);
    this.grew();
  }

  /** Rule 4. */
  private user(item: Item, creator: Creator): void {
    const from = sender(creator, this.images);
    this.ensure(`user:${from.key}`, from.label, from.tone, item.created);
    const part = item as unknown as { type: string; key: string } & Record<string, unknown>;
    const str = (key: string) => (typeof part[key] === "string" ? (part[key] as string) : "");
    switch (part.type) {
      case "user_text_content":
        this.appendText("text", str("text"), part.key);
        break;
      case "user_image_content":
        this.place(imageElement(str("data"), str("mimeType")));
        break;
      case "user_audio_content":
        this.place(audioElement(str("data"), str("mimeType")));
        break;
      case "user_resource": {
        const r = (part.resource ?? {}) as Record<string, unknown>;
        this.place(typeof r.text === "string" ? textElement(r.text) : resourceElement(String(r.uri ?? "resource")));
        break;
      }
      case "user_resource_link":
        this.place(resourceElement(`${str("name")} — ${str("uri")}`));
        break;
      default:
        this.place(jsonBlock(part));
    }
  }

  /** Rule 1: the open message if its role is `role`, else a new one with its label. */
  private ensure(role: string, label: string, tone: string, created: string): void {
    if (this.open?.role === role) return;
    this.close();
    const message = document.createElement("article");
    message.className = `message ${tone}`;
    const head = document.createElement("header");
    const name = document.createElement("span");
    name.className = "label";
    name.textContent = label;
    head.append(name, agoElement(created));
    const blocks = document.createElement("div");
    blocks.className = "blocks";
    message.append(head, blocks);
    this.body.append(message);
    this.open = { role, blocks };
  }

  /** Close the open message: whatever comes next starts its own. */
  private close(): void {
    this.open = null;
    this.text = null;
  }

  /** Put a finished block in the open message, ending its text block (rule 2). */
  private place(el: HTMLElement): void {
    this.text = null;
    this.open?.blocks.append(el);
    this.grew();
  }

  /** Rules 2 and 4: add text to the current block of its kind, or start one. */
  private appendText(kind: TextKind, text: string, key: string | null): void {
    if (this.text && this.text.kind === kind) {
      // Two user messages in one run keep a blank line between them.
      const gap = key !== null && this.text.key !== null && key !== this.text.key ? "\n\n" : "";
      this.text.block.buffer += gap + text;
      this.text.key = key;
      this.mark(this.text.block);
      return;
    }
    const el = document.createElement("div");
    let block: Growing;
    if (kind === "reasoning") {
      const details = document.createElement("details");
      details.className = "reasoning";
      const summary = document.createElement("summary");
      summary.textContent = "reasoning";
      const inner = document.createElement("div");
      details.append(summary, inner);
      el.append(details);
      // Built only while open, as the old viewer's disclosures are.
      block = { buffer: text, draw: () => details.open && inner.replaceChildren(textElement(block.buffer)) };
      details.addEventListener("toggle", () => block.draw());
    } else {
      el.className = kind === "refusal" ? "refusal" : "";
      block = { buffer: text, draw: () => el.replaceChildren(textElement(block.buffer)) };
    }
    this.open?.blocks.append(el);
    this.text = { kind, block, key };
    this.mark(block);
  }

  /** Rule 2: a tool call's card, made at its first fragment and grown by the rest. */
  private call(id: string, name: string, args: string): void {
    this.text = null;
    this.names.set(id, name);
    const known = this.calls.get(id);
    if (known) {
      known.args.buffer += args;
      this.mark(known.args);
      return;
    }
    const card = document.createElement("details");
    card.className = "call";
    const head = document.createElement("summary");
    const status = document.createElement("span");
    status.className = "status";
    status.textContent = "…";
    const title = document.createElement("span");
    title.className = "call-name";
    title.textContent = toolName(name, this.images);
    head.append(status, title);
    const body = document.createElement("div");
    body.className = "args";
    const response = document.createElement("div");
    response.className = "response";
    response.hidden = true;
    card.append(head, body, response);
    const block: Growing = {
      buffer: args,
      draw: () => {
        let parsed: unknown;
        try {
          parsed = block.buffer.trim() === "" ? undefined : JSON.parse(block.buffer);
        } catch {
          parsed = undefined;
        }
        if (parsed === undefined) body.textContent = block.buffer;
        else body.replaceChildren(jsonBlock(parsed));
      },
    };
    this.calls.set(id, { args: block, card, status, response });
    this.open?.blocks.append(card);
    this.mark(block);
  }

  /** Rule 7. */
  private mark(block: Growing): void {
    this.dirty.add(block);
    if (this.frame !== null) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = null;
      for (const b of this.dirty) b.draw();
      this.dirty.clear();
      this.grew();
    });
  }

  /** Stop drawing: a frame asked for is not drawn. */
  dispose(): void {
    if (this.frame !== null) cancelAnimationFrame(this.frame);
    this.frame = null;
    this.dirty.clear();
  }
}
