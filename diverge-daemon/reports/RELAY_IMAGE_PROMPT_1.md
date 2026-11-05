# The relay network, as one picture

An image prompt. The subject is how a relay hops from daemon to daemon,
each hop passing through exactly one provider. The facts are the
daemon's (2026-11-05); the style is TRON, top-down.

The picture is built as a **neon subway map**. A relay is one train
line, and its stations alternate between two kinds: big cyan hexagons
for daemons, and big white diamonds for providers. On a subway map a
line visits every station on it, so that framing puts the providers on
the route instead of beside it.

## Style

A flat, 2D, top-down TRON schematic: glowing neon vector lines on pure
black, with a faint cyan grid, heavy bloom and HUD tick marks around
the border. The type is a wide, glowing, geometric sci-fi font in all
caps. No real logos or brands.

## The orange line

- **The line:** one thick, glowing orange-red line, like a subway line
  or a lightcycle wall. It climbs across the frame as a **staircase**
  from bottom-left to top-right.
- **The steps:** the line is made of seven stations, evenly spaced along
  it like beads on a string. They alternate in type:

  | # | station | shape |
  |---|---|---|
  | 1 | `ALPHA` | cyan hexagon |
  | 2 | `P-02` | white diamond |
  | 3 | `BRAVO` | cyan hexagon |
  | 4 | `P-05` | white diamond |
  | 5 | `ORION` | cyan hexagon |
  | 6 | `P-07` | white diamond |
  | 7 | `NOVA` | cyan hexagon |

- **The pattern:** hexagon, diamond, hexagon, diamond, hexagon, diamond,
  hexagon. Each diamond sits exactly on the line, halfway between the
  two hexagons beside it, like a station on a subway line.
- **Size:** the diamonds are as large as the hexagons, so they read as
  stations, not as small dots.
- **The line inside the stations:** the line enters each station on one
  side and exits on the opposite side. Each diamond and each hexagon is
  lit up orange inside where the line runs through it.
- **Packets:** small white squares of light ride along the line.

## The stations

- **Daemons:** each hexagon has a magenta circular sigil at its center,
  like a fingerprint made of arcs, and a different one for every daemon.
  That sigil is the daemon's identity. Its name is written beside it in
  glowing caps.
- **Providers:** each diamond has a soft white halo and its label,
  `P-02`, `P-05` or `P-07`, beside it.

## The rest of the map

- **Spare providers:** around the orange line, dimmer extra white
  diamonds are scattered: other providers. Each hexagon on the line
  also has two thin, dim cyan **spur lines** branching off it to two of
  these extra diamonds. They are the daemon's other providers, unlit
  and unused by this route.
- **Distant daemons:** at the edges of the map sit more dim cyan
  hexagons, with their own dim spur lines to diamonds.
- **Brightness:** the dim spurs show that every daemon has several
  providers, while the bright orange line uses one of them per step.

## Text

- **The title**, huge and glowing at the top: **THE RELAY**.
- **Beneath it**, in thin caps: `DAEMON ▸ PROVIDER ▸ DAEMON ▸ PROVIDER ▸ DAEMON`.
- **A legend** sits bottom-right in a small HUD panel:
  - a cyan hexagon, `DAEMON`;
  - a white diamond, `PROVIDER`;
  - a magenta sigil, `IDENTITY`;
  - an orange line, `RELAY`.
- **A status readout** sits bottom-left: `ROUTE LOCKED — ONE PROVIDER PER HOP`.

## Composition

A flat overhead view, 16:9. The orange staircase with its seven
alternating stations is the bright heart of the image, and the dim
mesh fills the rest of the board.
