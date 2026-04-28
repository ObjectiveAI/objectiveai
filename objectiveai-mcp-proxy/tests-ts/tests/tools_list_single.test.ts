import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('one upstream → tools come back as `<server-name>_<tool>`', async () => {
  rig = await startRig([
    {
      serverName: 'fs',
      initialTools: [
        { name: 'Read', behavior: { kind: 'echo' } },
        { name: 'Write', behavior: { kind: 'echo' } },
      ],
    },
  ]);
  const client = await rig.connectClient({
    'X-MCP-Servers': rig.xMcpServers(),
  });
  const result = await client.listTools();
  const names = new Set(result.tools.map(t => t.name));
  expect(names).toEqual(new Set(['fs_Read', 'fs_Write']));
  await client.close();
});
