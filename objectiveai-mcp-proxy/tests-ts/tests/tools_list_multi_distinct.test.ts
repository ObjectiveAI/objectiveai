import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('two upstreams with distinct names → both prefix sets concatenated', async () => {
  rig = await startRig([
    {
      serverName: 'alpha',
      initialTools: [
        { name: 'Read', behavior: { kind: 'echo' } },
        { name: 'Write', behavior: { kind: 'echo' } },
      ],
    },
    {
      serverName: 'beta',
      initialTools: [{ name: 'Ping', behavior: { kind: 'echo' } }],
    },
  ]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['alpha_Read', 'alpha_Write', 'beta_Ping']));
  await client.close();
});
