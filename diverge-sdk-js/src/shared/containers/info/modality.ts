// One kind of content, beside text, an agent may take.

import { z } from "zod";

/**
 * One of the kinds of content a `Modalities` answers for, by name:
 * `"image"`, `"audio"`, `"file"`, ordered so.
 */
export const Modality = z.enum(["image", "audio", "file"]);
export type Modality = z.infer<typeof Modality>;
