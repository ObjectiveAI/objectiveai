import { describe, expect, it } from "vitest";
import { z } from "zod";
import { CliStream } from "./cliStream";

const ErrorEnvelope = z.object({
  type: z.literal("error"),
  message: z.string(),
});
const Leaf = z.object({ hello: z.string() });
const Union = z.union([ErrorEnvelope, Leaf]);

async function* lines(...values: unknown[]): AsyncGenerator<unknown> {
  for (const v of values) yield v;
}

describe("CliStream", () => {
  it("unwraps externally-tagged aggregate layers until the schema parses", async () => {
    const stream = new CliStream(
      lines({ Agents: { Spawn: { hello: "world" } } }),
      Union,
    );
    expect(await stream.toArray()).toEqual([{ hello: "world" }]);
  });

  it("yields error envelopes as union members without unwrapping confusion", async () => {
    const stream = new CliStream(
      lines(
        { type: "error", message: "boom" },
        { Agents: { Spawn: { hello: "after" } } },
      ),
      Union,
    );
    expect(await stream.toArray()).toEqual([
      { type: "error", message: "boom" },
      { hello: "after" },
    ]);
  });

  it("never yields the synthetic end marker", async () => {
    const stream = new CliStream(
      lines({ hello: "a" }, { type: "end" }),
      Union,
    );
    expect(await stream.toArray()).toEqual([{ hello: "a" }]);
  });

  it("first() resolves the first item and discards the rest", async () => {
    let pulled = 0;
    const source: AsyncIterable<unknown> = {
      async *[Symbol.asyncIterator]() {
        pulled++;
        yield { hello: "first" };
        pulled++;
        yield { hello: "second" };
      },
    };
    const stream = new CliStream(source, Union);
    expect(await stream.first()).toEqual({ hello: "first" });
    expect(pulled).toBe(1);
  });

  it("first() is undefined when the stream ends without items", async () => {
    const stream = new CliStream(lines({ type: "end" }), Union);
    expect(await stream.first()).toBeUndefined();
  });

  it("throws the original-value ZodError when nothing parses", async () => {
    const stream = new CliStream(lines({ a: 1, b: 2 }), Union);
    await expect(stream.toArray()).rejects.toThrow();
  });
});
