// Writing to an agent from its log: the box under the log, the files
// attached, and the messages sent and not yet taken.

import type { daemon } from "@diverge/sdk";
import type { Route } from "../lib/wire";
import { type Attachment, attachment, content } from "./attach";
import { contentElement } from "./media";
import { keyOf, outbox } from "./outbox";

/** The tallest the text box grows, in pixels, before it scrolls. */
const TALLEST = 160;

/**
 * The composer of one agent's sheet, from the moment it is made until
 * `dispose`.
 *
 * The rules:
 *
 * 1. The pending strip (`strip`) stands at the log's bottom: one greyed
 *    preview per message in the agent's outbox, oldest first, its
 *    blocks drawn as the log draws them (`contentElement`), each with
 *    ✕, which drops it (`outbox.drop`: a waiting message cancelled, a
 *    failed one taken out). A failed message is drawn red, with why.
 * 2. The composer (`root`) stands under the log: the files attached,
 *    then a text box, 📎 and send.
 * 3. Each file attached is a chip: an image its thumbnail, a sound ♪
 *    and its name, any other file 📄 and its name; each with ✕, which
 *    takes it off. Files are read as they are attached and kept in that
 *    order, whatever order their reads end in; a file that cannot be
 *    read is left out.
 * 4. 📎 picks files with the system's dialog, any number at once;
 *    pasting files, a screenshot among them, attaches them too.
 * 5. The text box grows with its text, up to `TALLEST`. Enter sends;
 *    Shift+Enter breaks the line. Its keys are its own: none reaches
 *    the board's keys beneath.
 * 6. Send is pressable while there is text that is not blank or a file
 *    attached, and no file is still being read. Sending puts the text,
 *    then the files (`content`), in the outbox (`outbox.send`) and
 *    empties the text box and the chips; the message appears in the
 *    strip.
 * 7. `dispose` stops listening to the outbox and takes the strip and
 *    the composer out of the page; the messages stay in the outbox.
 */
export class Composer {
  /** Rule 1. */
  readonly strip: HTMLElement;
  /** Rule 2. */
  readonly root: HTMLElement;
  private readonly chips: HTMLElement;
  private readonly text: HTMLTextAreaElement;
  private readonly send: HTMLButtonElement;
  private readonly key: string;
  private readonly unwatch: () => void;
  /** The files attached, in order: each a read, then what it read, or `null` when it could not be. */
  private attached: { file: File; read: Attachment | null | undefined }[] = [];

  constructor(
    private readonly route: Route,
    private readonly agent: daemon.reference.Agent,
    entity: string,
  ) {
    this.key = keyOf(route, entity);
    // Rule 1.
    this.strip = document.createElement("div");
    this.strip.className = "pending";
    // Rule 2.
    this.root = document.createElement("div");
    this.root.className = "composer";
    this.chips = document.createElement("div");
    this.chips.className = "chips";
    const row = document.createElement("div");
    row.className = "row";
    this.text = document.createElement("textarea");
    this.text.rows = 1;
    this.text.placeholder = "Message the agent";
    const picker = document.createElement("input");
    picker.type = "file";
    picker.multiple = true;
    picker.hidden = true;
    const clip = document.createElement("button");
    clip.type = "button";
    clip.className = "clip";
    clip.textContent = "📎";
    clip.title = "Attach files";
    this.send = document.createElement("button");
    this.send.type = "button";
    this.send.className = "send";
    this.send.textContent = "Send";
    row.append(this.text, clip, this.send, picker);
    this.root.append(this.chips, row);
    // Rule 4.
    clip.addEventListener("click", () => picker.click());
    picker.addEventListener("change", () => {
      this.attach([...(picker.files ?? [])]);
      picker.value = "";
    });
    this.text.addEventListener("paste", (e) => {
      const files = [...(e.clipboardData?.files ?? [])];
      if (files.length === 0) return;
      if (!e.clipboardData?.types.includes("text/plain")) e.preventDefault();
      this.attach(files);
    });
    // Rule 5.
    this.text.addEventListener("input", () => {
      this.grow();
      this.ready();
    });
    this.text.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
        e.preventDefault();
        this.sent();
      }
    });
    for (const type of ["keydown", "keyup"] as const) this.root.addEventListener(type, (e) => e.stopPropagation());
    // Rule 6.
    this.send.addEventListener("click", () => this.sent());
    this.ready();
    // Rule 1.
    this.unwatch = outbox.watch(this.key, () => this.drawStrip());
    this.drawStrip();
  }

  /** Rule 7. */
  dispose(): void {
    this.unwatch();
    this.strip.remove();
    this.root.remove();
  }

  /** Rule 3: attach files, each read in turn, kept in the order given. */
  private attach(files: File[]): void {
    for (const file of files) {
      const slot: { file: File; read: Attachment | null | undefined } = { file, read: undefined };
      this.attached.push(slot);
      void attachment(file).then(
        (read) => (slot.read = read),
        () => (slot.read = null),
      ).then(() => {
        if (slot.read === null) this.attached = this.attached.filter((s) => s !== slot);
        this.drawChips();
      });
    }
    this.drawChips();
  }

  /** Rule 3. */
  private drawChips(): void {
    this.chips.replaceChildren(
      ...this.attached.map((slot) => {
        const chip = document.createElement("span");
        chip.className = "chip";
        chip.title = slot.file.name;
        const read = slot.read;
        if (read?.kind === "image" && read.block.type === "image") {
          const img = document.createElement("img");
          img.src = `data:${read.mime};base64,${read.block.data}`;
          img.alt = read.name;
          chip.append(img);
        } else {
          const name = document.createElement("span");
          name.className = "name";
          name.textContent = `${read === undefined ? "…" : read?.kind === "audio" ? "♪" : "📄"} ${slot.file.name}`;
          chip.append(name);
        }
        const off = document.createElement("button");
        off.type = "button";
        off.textContent = "✕";
        off.title = "Remove";
        off.addEventListener("click", () => {
          this.attached = this.attached.filter((s) => s !== slot);
          this.drawChips();
        });
        chip.append(off);
        return chip;
      }),
    );
    this.chips.hidden = this.attached.length === 0;
    this.ready();
  }

  /** Rule 5: the text box as tall as its text, up to `TALLEST`. */
  private grow(): void {
    this.text.style.height = "auto";
    this.text.style.height = `${Math.min(this.text.scrollHeight, TALLEST)}px`;
  }

  /** Rule 6: whether send may be pressed. */
  private ready(): void {
    const reading = this.attached.some((slot) => slot.read === undefined);
    const something = this.text.value.trim() !== "" || this.attached.length > 0;
    this.send.disabled = reading || !something;
  }

  /** Rule 6: send what is written and attached, and empty both. */
  private sent(): void {
    if (this.send.disabled) return;
    const files = this.attached.flatMap((slot) => (slot.read ? [slot.read] : []));
    outbox.send(this.route, this.agent, this.key, content(this.text.value, files));
    this.text.value = "";
    this.attached = [];
    this.grow();
    this.drawChips();
  }

  /** Rule 1: the strip, drawn again from the outbox. */
  private drawStrip(): void {
    this.strip.replaceChildren(
      ...outbox.of(this.key).map((message) => {
        const el = document.createElement("div");
        el.className = `waiting ${message.state}`;
        const blocks = document.createElement("div");
        blocks.className = "blocks";
        for (const block of message.content) blocks.append(contentElement(block as { type: string } & Record<string, unknown>));
        if (message.why) {
          const why = document.createElement("div");
          why.className = "why";
          why.textContent = message.why;
          blocks.append(why);
        }
        const off = document.createElement("button");
        off.type = "button";
        off.textContent = "✕";
        off.title = message.state === "waiting" ? "Cancel" : "Dismiss";
        off.addEventListener("click", () => outbox.drop(this.key, message.id));
        el.append(blocks, off);
        return el;
      }),
    );
  }
}
