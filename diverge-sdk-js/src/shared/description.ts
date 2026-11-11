// One short line of plain text: a daemon's announcement, as a
// provider's `daemons::list` lists it, and an image's description.

import { z } from "zod";

/** The most characters a description holds, counted as Unicode scalar values. */
export const DESCRIPTION_CHARACTERS = 132;

/**
 * One short line of plain text. It is what a daemon's announcement is
 * made of — the text of its `daemons::announce`, which a provider's
 * `daemons::list` lists beside its identity — and what an image's
 * description is made of. The rules:
 *
 * 1. Plain text. One line: no control character — no newline, no
 *    tab — anywhere in it.
 * 2. Short. At least one character, and at most
 *    `DESCRIPTION_CHARACTERS`, counted as Unicode scalar values.
 * 3. Trimmed. No whitespace at its start or its end, so that two
 *    descriptions that read the same are the same.
 *
 * A string that breaks a rule is no description: it does not parse.
 */
export const Description = z
  .string()
  .refine((text) => !/\p{Cc}/u.test(text), { message: "a description is one line: it holds no control character" })
  .refine(
    (text) => {
      const characters = [...text].length;
      return characters >= 1 && characters <= DESCRIPTION_CHARACTERS;
    },
    { message: `a description is 1 to ${DESCRIPTION_CHARACTERS} characters` },
  )
  .refine((text) => text === text.trim(), { message: "a description has no whitespace at its start or its end" });
export type Description = z.infer<typeof Description>;
