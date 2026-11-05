# The relay network, as one picture

An image prompt. The subject is how a relay hops from daemon to daemon
across the network, each hop passing through exactly one provider. The
facts are the daemon's (2026-11-05); the style is TRON, seen from
directly above.

## Style

Flat, 2D and top-down, like the ENCOM map on a TRON war-room table: a
glowing neon schematic on pure black, drawn as a circuit board seen
from orbit.
- **Light:** everything is made of razor-thin vector lines with heavy
  bloom. There is no perspective, no shading and no texture, only light
  on black.
- **The grid:** a faint cyan grid covers the whole frame. Fine hex
  patterns and tick marks run along the edges, like a targeting HUD.
- **Palette:**
  - cyan for daemons;
  - white for providers;
  - a hot orange-red for the relay;
  - magenta for identities.
- **Type:** a sharp, wide, geometric sci-fi font in all caps, glowing,
  with wide letter spacing. No real logos or brands.

## The network

- **Daemons:** six to eight daemons are spread across the board. Each
  is a cyan **hexagon**, a bold glowing outline with a smaller hexagon
  nested inside and a slowly rotating ring of segmented dashes around
  it.
- **Identities:** at the center of each daemon sits a magenta **identity
  glyph**, a small circular sigil like a fingerprint made of
  concentric arcs, different for every daemon. Each daemon carries a
  short name tag beside it in glowing caps: `ALPHA`, `BRAVO`, `ORION`,
  `VEGA`, `NOVA`, `ECHO`.
- **Providers:** white **diamonds** stand between the daemons, each
  with a pulsing halo. A few carry tiny tags: `PROVIDER 01`,
  `PROVIDER 02`, and so on.
- **Tethers:** every daemon is wired to two, three or four of the
  nearest providers by thin dim cyan traces with right-angle bends, like
  circuit-board traces. Many providers are wired to several daemons. The
  result is a dense, glowing mesh: every daemon has several providers,
  and the providers are the junctions where daemons meet.

## The relay

- **The run:** one route blazes across the mesh in hot orange-red, as
  thick as a lightcycle wall and with an intense glow. It starts at
  `ALPHA` near the bottom-left and ends at `NOVA` at the top-right.
- **The path:** the route is made of exactly six straight segments.
  Every segment runs between one hexagon and one diamond, never between
  two hexagons, so the route **touches a white provider diamond between
  every pair of daemons**. In order:
  1. `ALPHA` ⬡ to the diamond `PROVIDER 02` ◆.
  2. `PROVIDER 02` ◆ to `BRAVO` ⬡.
  3. `BRAVO` ⬡ to the diamond `PROVIDER 05` ◆.
  4. `PROVIDER 05` ◆ to `ORION` ⬡.
  5. `ORION` ⬡ to the diamond `PROVIDER 07` ◆.
  6. `PROVIDER 07` ◆ to `NOVA` ⬡.
- **Through the providers:** the route passes visibly **through the
  center** of each of those three diamonds, entering one side and
  leaving the other. Each of the three diamonds blazes orange-white
  where the route passes through it, the brightest points in the image.
  No orange line ever links two hexagons directly.
- **One provider per hop:** each daemon on the route is also wired to
  two or three other diamonds by dim cyan traces. The route takes
  exactly one of them, and the others stay dim.
- **Packets:** small bright squares of light ride along the orange
  route, leaving motion streaks.
- **Each hop:** where the route enters a daemon, the daemon's hexagon
  flares orange and its identity glyph pulses magenta, a handshake.
  Tiny glowing labels mark the order along the path: `HOP 1`, `HOP 2`,
  `HOP 3`.
- **The route's tag:** small text runs beside the route, like a flight
  path readout: `ALPHA ▸ P02 ▸ BRAVO ▸ P05 ▸ ORION ▸ P07 ▸ NOVA`.

## Text

- **The title** is huge and glowing across the top: **THE RELAY**.
  Beneath it, a thin line reads: `DAEMON ▸ PROVIDER ▸ DAEMON ▸ PROVIDER ▸ …`
- **A legend** sits in the bottom-right corner, in a small HUD panel:
  - ⬡ `DAEMON`
  - ◆ `PROVIDER`
  - ◉ `IDENTITY`
  - an orange line, `ACTIVE ROUTE`
- **A status readout** sits in the bottom-left corner: `ROUTE LOCKED —
  4 DAEMONS — 3 PROVIDERS — ONE PER HOP`.

## Composition

- **Top-down and flat:** the view is exactly overhead.
- **The diagonal:** the orange route cuts a bold diagonal from
  bottom-left to top-right. The dim mesh fills the rest of the board,
  so the eye goes straight to the route.
- **The feel:** high contrast and crisp, a mission-control screen at
  the moment a route locks in.
- **Aspect ratio:** 16:9.
