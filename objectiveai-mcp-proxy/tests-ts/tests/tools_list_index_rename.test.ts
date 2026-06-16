import { afterEach, expect, test } from 'vitest';

import { startRig, type Rig } from '../src/rig.js';

// Routing-prefix scheme: each upstream is keyed by a `_`/`.`-free prefix
// derived from its server name, escalating only on collision:
// `name` -> `name-version` -> `name-version-index`. (`_` and `.` in the
// name/version are normalized to `-`.)

let rig: Rig | undefined;
afterEach(async () => {
  await rig?.cleanup();
  rig = undefined;
});

test('distinct names use a bare normalized prefix', async () => {
  rig = await startRig([
    { serverName: 'fs', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
    { serverName: 'my.web_search', initialTools: [{ name: 'Query', behavior: { kind: 'echo' } }] },
  ]);
  const client = await rig.connectClient({ 'X-MCP-Servers': rig.xMcpServers() });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['fs_Read', 'my-web-search_Query']));
  await client.close();
});

test('same name, different version disambiguates by version', async () => {
  rig = await startRig([
    { serverName: 'fs', serverVersion: '1.0', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
    { serverName: 'fs', serverVersion: '2.0', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
  ]);
  const client = await rig.connectClient({ 'X-MCP-Servers': rig.xMcpServers() });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['fs-1-0_Read', 'fs-2-0_Read']));
  await client.close();
});

test('same name and version disambiguates by index', async () => {
  // Indices are url-sorted and not predictable from the random ports, but
  // both upstreams carry identical tools so the set is stable either way.
  rig = await startRig([
    { serverName: 'fs', serverVersion: '9.9', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
    { serverName: 'fs', serverVersion: '9.9', initialTools: [{ name: 'Read', behavior: { kind: 'echo' } }] },
  ]);
  const client = await rig.connectClient({ 'X-MCP-Servers': rig.xMcpServers() });
  const names = new Set((await client.listTools()).tools.map(t => t.name));
  expect(names).toEqual(new Set(['fs-9-9-0_Read', 'fs-9-9-1_Read']));
  await client.close();
});
