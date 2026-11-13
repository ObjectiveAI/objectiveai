// One question of a structured decision.

import { z } from "zod";
import { ContentBlockSchema } from "@modelcontextprotocol/core";
import { ChoiceOption } from "./choice_option.js";

/**
 * One question: its id, its kind, what it asks, and what its kind
 * adds. Tagged by `type`: `"noul"`, `"choice"`, `"score"`. Every kind
 * has `id` and `instructions`; a choice adds `options` and a score
 * adds `levels`, and no kind has any other member. The id is the
 * caller's, and is the key its answer comes back under.
 */
export const Question = z.discriminatedUnion("type", [
  z.strictObject({
    /** A single number: nothing beside the id and the instructions. */
    type: z.literal("noul"),
    /** The question's id: what its answer is keyed by. */
    id: z.string(),
    /** What is asked, as content parts, in order. */
    instructions: z.array(ContentBlockSchema),
  }),
  z.strictObject({
    /** One among options. */
    type: z.literal("choice"),
    /** The question's id: what its answer is keyed by. */
    id: z.string(),
    /** What is asked, as content parts, in order. */
    instructions: z.array(ContentBlockSchema),
    /** The options, in order. */
    options: z.array(ChoiceOption),
  }),
  z.strictObject({
    /** A place along levels. */
    type: z.literal("score"),
    /** The question's id: what its answer is keyed by. */
    id: z.string(),
    /** What is asked, as content parts, in order. */
    instructions: z.array(ContentBlockSchema),
    /** The levels, in order, each content parts describing it. */
    levels: z.array(z.array(ContentBlockSchema)),
  }),
]);
export type Question = z.infer<typeof Question>;
