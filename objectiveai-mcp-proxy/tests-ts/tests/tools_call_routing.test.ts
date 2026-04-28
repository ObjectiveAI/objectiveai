import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('tools/call routes to the upstream matching the prefix', async () => {
  rig = await startRig([
    {
      serverName: 'alpha',
      initialTools: [
        { name: 'say', behavior: { kind: 'static', reply: 'from-alpha' } },
      ],
    },
    {
      serverName: 'beta',
      initialTools: [
        { name: 'say', behavior: { kind: 'static', reply: 'from-beta' } },
      ],
    },
  ]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });

  const a = await client.callTool({ name: 'alpha_say', arguments: {} });
  const b = await client.callTool({ name: 'beta_say', arguments: {} });

  expect(firstText(a)).toBe('from-alpha');
  expect(firstText(b)).toBe('from-beta');

  await client.close();
});

function firstText(result: { content: unknown }): string {
  for (const block of result.content as Array<{ type: string; text?: string }>) {
    if (block.type === 'text' && typeof block.text === 'string') return block.text;
  }
  throw new Error(`no text content in ${JSON.stringify(result)}`);
}
