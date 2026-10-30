// A tool's log, folded into messages as its items arrive.

import type { daemon, shared } from "@diverge/sdk";
import { agoElement } from "./ago";
import { type Card, answer, callCard } from "./card";
import { jsonBlock } from "./json";
import { contentElement } from "./media";
import { caller } from "./sender";

type Item = daemon.tool_log.ItemWrapper;
type Caller = shared.mcp.attestation.Caller;

/** The message being added to: whose asks it holds, as a key, and where its blocks go. */
type Open = { key: string; blocks: HTMLElement };

/**
 * The log of one tool, drawn into `body` as messages.
 *
 * The rules:
 *
 * 1. A message is one caller's run: its asks, and the notifications
 *    the tool sent on its session, labelled by the caller (`caller`)
 *    once, at its head, with when it began. An ask or a notification
 *    of the same caller joins the open message; another caller's
 *    starts a new one.
 * 2. A call is a card: the tool's own name for what was called, over
 *    its arguments as JSON. Its result fills the card, matched by the
 *    call's index — its content drawn block by block, or its
 *    structured content when it has none — marked done, or failed when
 *    the tool said it is an error; a call error fills it red.
 * 3. A read is a card naming the resource, filled by its contents or
 *    its error the same way.
 * 4. An answer whose ask the log has not shown is a message of its own.
 * 5. A notification is a block of its recipient's run (rule 1),
 *    showing its JSON. An error is a red message of its own, and
 *    closes the open message.
 * 6. `active` and `inactive` draw nothing and close nothing: they only
 *    light and dim the sheet's live dot.
 * 7. Every item comes whole and is drawn as it comes; `grew` is called
 *    after every drawing.
 */
export class ToolFold {
  private open: Open | null = null;
  private readonly cards = new Map<number, Card>();

  constructor(
    private readonly body: HTMLElement,
    private readonly grew: () => void,
    private readonly active: (on: boolean) => void,
  ) {}

  /** One item of the log, in order. */
  push(item: Item): void {
    switch (item.type) {
      case "call": {
        // Rules 1 and 2.
        const card = this.ask(item, item.caller, item.name);
        card.args.replaceChildren(jsonBlock(item.arguments ?? {}));
        break;
      }
      case "read":
        // Rules 1 and 3.
        this.ask(item, item.caller, `read · ${item.uri}`);
        break;
      case "call_result": {
        const content = (item.result.content ?? []) as ({ type: string } & Record<string, unknown>)[];
        const drawn = content.map((block) => contentElement(block));
        if (content.length === 0 && item.result.structuredContent !== undefined) drawn.push(jsonBlock(item.result.structuredContent));
        this.fill(item, item.call, drawn, item.result.isError === true);
        break;
      }
      case "read_result": {
        const drawn = item.result.contents.map((resource) => contentElement({ type: "resource", resource }));
        this.fill(item, item.read, drawn, false);
        break;
      }
      case "call_error":
        this.fill(item, item.call, [jsonBlock(item.error)], true);
        break;
      case "read_error":
        this.fill(item, item.read, [jsonBlock(item.error)], true);
        break;
      case "notification": {
        // Rules 1 and 5.
        const block = document.createElement("div");
        block.className = "notify";
        block.append(jsonBlock(item.notification));
        this.join(item, item.to).append(block);
        this.grew();
        break;
      }
      case "error":
        this.alone(item, "error", "error", jsonBlock(item.error), true);
        break;
      case "active":
      case "inactive":
        // Rule 6.
        this.active(item.type === "active");
        break;
    }
  }

  /** A line of the sheet's own, outside any message: the log ended, was refused, or failed. */
  note(text: string): void {
    this.open = null;
    const el = document.createElement("div");
    el.className = "note";
    el.textContent = text;
    this.body.append(el);
    this.grew();
  }

  /** Nothing to stop: every item is drawn as it comes. */
  dispose(): void {}

  /** Rule 1: an ask's card, in its caller's message. */
  private ask(item: Item, who: Caller, title: string): Card {
    const card = callCard(title);
    this.cards.set(item.logs_index, card);
    this.join(item, who).append(card.card);
    this.grew();
    return card;
  }

  /** Rule 1: the open message if it is `who`'s, else a new one; where its blocks go. */
  private join(item: Item, who: Caller): HTMLElement {
    const from = caller(who);
    if (this.open?.key !== from.key) this.open = { key: from.key, blocks: this.message(from.label, from.tone, item.created) };
    return this.open.blocks;
  }

  /** Rules 2 to 4: an answer into its ask's card, or a message of its own. */
  private fill(item: Item, ask: number, drawn: HTMLElement[], failed: boolean): void {
    const card = this.cards.get(ask);
    if (card) {
      answer(card, drawn, failed);
      this.grew();
      return;
    }
    const blocks = this.message("answer", "tool", item.created, failed);
    blocks.append(...drawn);
    this.open = null;
    this.grew();
  }

  /** Rule 5: one block, a message of its own. */
  private alone(item: Item, label: string, tone: string, el: HTMLElement, failed: boolean): void {
    this.open = null;
    this.message(label, tone, item.created, failed).append(el);
    this.grew();
  }

  /** A message with its label and when it began; where its blocks go. */
  private message(label: string, tone: string, created: string, failed = false): HTMLElement {
    const message = document.createElement("article");
    message.className = `message ${tone}${failed ? " failed" : ""}`;
    const head = document.createElement("header");
    const name = document.createElement("span");
    name.className = "label";
    name.textContent = label;
    head.append(name, agoElement(created));
    const blocks = document.createElement("div");
    blocks.className = "blocks";
    message.append(head, blocks);
    this.body.append(message);
    return blocks;
  }
}
