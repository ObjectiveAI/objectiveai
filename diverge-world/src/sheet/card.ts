// A call's card: what was asked, a mark, and the answer beneath.

/** A card's parts: the card, its mark, what was asked, and where the answer goes. */
export type Card = { card: HTMLElement; status: HTMLElement; args: HTMLElement; response: HTMLElement };

/**
 * A card, closed until opened: its mark — pending until answered — and
 * `title`, over what was asked and the answer, hidden until there is one.
 */
export function callCard(title: string): Card {
  const card = document.createElement("details");
  card.className = "call";
  const head = document.createElement("summary");
  const status = document.createElement("span");
  status.className = "status";
  status.textContent = "…";
  const name = document.createElement("span");
  name.className = "call-name";
  name.textContent = title;
  head.append(status, name);
  const args = document.createElement("div");
  args.className = "args";
  const response = document.createElement("div");
  response.className = "response";
  response.hidden = true;
  card.append(head, args, response);
  return { card, status, args, response };
}

/** The answer drawn into its card, marked done, or failed and red. */
export function answer(card: Card, drawn: HTMLElement[], failed: boolean): void {
  card.response.replaceChildren(...drawn);
  card.response.hidden = false;
  card.status.textContent = failed ? "✕" : "✓";
  card.card.classList.toggle("failed", failed);
}
