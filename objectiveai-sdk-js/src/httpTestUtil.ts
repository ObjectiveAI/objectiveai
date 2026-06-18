import { describe, it, expect } from "vitest";
import { ObjectiveAI } from "./client";
import { rounded } from "./mergeTestUtil";
import type { Stream } from "./stream";
import * as fs from "fs";
import * as path from "path";

const address = process.env.OBJECTIVEAI_ADDRESS;

export const httpTestClient = address
  ? new ObjectiveAI({ address })
  : null;

export function loadSnapshot<U>(snapshotsDir: string, name: string): U {
  return JSON.parse(fs.readFileSync(path.join(snapshotsDir, `${name}.json`), "utf-8"));
}

export interface HttpTestCase {
  snapshot: string;
  body: Record<string, unknown>;
}

export interface HttpTestSuiteOptions<Chunk, Unary> {
  /** describe() label */
  name: string;
  /** Absolute path to the directory containing snapshot JSON files */
  snapshotsDir: string;
  /** The exported HTTP function from the corresponding http.ts module */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  fn: (client: ObjectiveAI, body: any) => Promise<Unary | Stream<Chunk>>;
  /** Merge an accumulated chunk with a new chunk */
  merge: (acc: Chunk, chunk: Chunk) => [Chunk, boolean];
  /** Convert a fully-accumulated chunk into a unary response */
  chunkToUnary: (acc: Chunk) => Unary;
  /** WASM normalize for test comparison (zeros non-deterministic fields) */
  normalize: (unary: Unary) => Unary;
  /** The individual test cases */
  cases: HttpTestCase[];
}

export function httpTestSuite<Chunk, Unary>(opts: HttpTestSuiteOptions<Chunk, Unary>) {
  async function postUnary(body: Record<string, unknown>): Promise<Unary> {
    return opts.fn(httpTestClient!, { ...body, stream: false }) as Promise<Unary>;
  }

  async function postStreaming(body: Record<string, unknown>): Promise<Unary> {
    const stream = (await opts.fn(httpTestClient!, { ...body, stream: true })) as Stream<Chunk>;
    let acc: Chunk | null = null;
    for await (const chunk of stream) {
      if (acc === null) {
        acc = chunk;
      } else {
        [acc] = opts.merge(acc, chunk);
      }
    }
    expect(acc).not.toBeNull();
    return opts.chunkToUnary(acc!);
  }

  describe.skipIf(!address)(opts.name, () => {
    for (const c of opts.cases) {
      it(`${c.snapshot} (unary)`, async () => {
        const expected = rounded(loadSnapshot<Unary>(opts.snapshotsDir, c.snapshot));
        expect(rounded(opts.normalize(await postUnary(c.body)))).toEqual(expected);
      });

      it(`${c.snapshot} (streaming)`, async () => {
        const expected = rounded(loadSnapshot<Unary>(opts.snapshotsDir, c.snapshot));
        expect(rounded(opts.normalize(await postStreaming(c.body)))).toEqual(expected);
      });
    }
  });
}
