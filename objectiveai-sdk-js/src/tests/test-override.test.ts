// Tests Zod's toJSONSchema `override` callback to verify that referenced schemas
// can be replaced with `$ref` pointers. This is the mechanism the SDK uses to strip
// inline `$defs` and rewrite nested types as bare `$ref` titles — matching the same
// ref-rewriting the JSON schema builder does on the Rust side.
//
// Expected output: the inner "inner.Title" schema is replaced with { "$ref": "inner.Title" }
// while the root "outer.Title" schema keeps its full definition.

import { describe, it, expect } from "vitest";
import { z, toJSONSchema } from "zod";

describe("Zod toJSONSchema override", () => {
  it("replaces inner titled schemas with $ref pointers", () => {
    const Inner = z.string().meta({ title: "inner.Title" });
    const Outer = z.object({ x: Inner }).meta({ title: "outer.Title" });
    const allTitles = new Set(["inner.Title", "outer.Title"]);

    const result = toJSONSchema(Outer, {
      reused: "inline",
      override(ctx) {
        const js = ctx.jsonSchema;
        if (!js || typeof js !== "object") return;
        if (
          "title" in js &&
          typeof js.title === "string" &&
          allTitles.has(js.title) &&
          js.title !== "outer.Title"
        ) {
          const title = js.title;
          for (const key of Object.keys(js)) delete js[key];
          (js as Record<string, unknown>).$ref = title;
        }
      },
    });

    expect(result.properties).toBeDefined();
    const props = result.properties as Record<string, unknown>;
    expect(props.x).toEqual({ $ref: "inner.Title" });
  });
});
