# Image prompt: two daemons, one tool between them, three providers underneath

An image prompt for one picture of what landed on 2026-10-10: a tool
one daemon runs, joined by an agent of another daemon through a
provider, every container of either daemon powered by one of three
providers. The picture is a poster and nothing but the poster, in the
spirit of a wall that somebody dangerous and sane has been working on
for a long time. Almost no text. Structure and colour carry the
meaning.

Who connects to whom is fixed in the tables below. How it is drawn is
the image's to decide, within the tone.

## Prompt

### The tone

The poster is the whole frame. There is no wall behind it, no
corkboard, no table edge, no room: the paper runs to all four edges
of the image, as if the camera were pressed flat against it. What
you are looking at is the thing itself.

The tone is anarchist, and it is calm. Not rage: certainty. The hand
that made this is steady. Black spray-paint stencils with soft
over-spray, xeroxed grain, photocopied photographs pasted down and
re-photocopied until the blacks crush, red marker that has bled a
little into the paper, torn edges, one sentence that reads like an
instruction left for whoever finds the wall after. The palette is
cold: ink black, bone white, newsprint grey, one red, and three
colours of light that are described below. It should feel like a
samizdat broadsheet, like a pirate-radio schedule, like a diagram
someone pinned up for people who already know what it means. No
people, no faces, no logos, no slogans but the one. The image is free
in composition, texture, how worn the paper is, how the stencils were
cut, how the light falls across it. It is not free in who is joined
to whom.

**Across the very top, stencilled in white spray paint, slightly
misregistered, the only sentence on the poster: `DO NOT BE AFRAID.`**

The poster falls into three horizontal regions, top to bottom,
separated however the image likes: duct tape, a torn seam, a band of
black.

### The light, which is the providers

Three providers stand across the bottom of the poster. Each is a
piece of heavy infrastructure, photocopied and pasted down, and each
is lit from behind by a glow of its own colour, a halo that bleeds
out into the paper around it as if the thing were switched on in the
dark:

| Provider | Photograph | Glow |
|---|---|---|
| `I` | a substation transformer, insulators and cooling fins | amber, like a sodium lamp |
| `II` | a bank of rack servers, cable spaghetti, one lit LED | cold cyan |
| `III` | a radio mast against a flat sky, guy wires | violet, like a UV tube |

Each is stamped with its stencilled numeral and nothing else.

**That glow is how power is drawn.** Nothing runs from a provider to
anything. Instead, every agent, tool and dependency tool in the upper
regions sits in a halo of exactly one of the three colours, the same
glow its provider has, bleeding out behind it the same way. An amber
thing runs on the transformer; a cyan thing runs on the servers; a
violet thing runs on the mast. The three colours are scattered across
the two daemons so that no cluster is one colour: the eye reads the
daemons as structure and the light as a second, independent
structure laid over it. No lines, no arrows, no wires to the
providers. Only the light.

### The two shapes

- **A cabinet.** Every agent and every tool is a photocopied
  photograph of a locked steel cabinet, the same size, its codename
  stencilled across the door. An agent's door carries a stencilled
  **eye**; a tool's door carries a stencilled **gear**. That stamp
  is the only thing that tells an agent from a tool.
- **A tag.** Every dependency tool is a luggage tag, a third the size
  of a cabinet, hanging under its agent on a short red string. A tag
  carries a **glyph** scratched into it in black marker and a
  four-character hex fragment in tiny monospace, and nothing else.
  No tag has a name.

Red string is the only line on the poster, and it means one thing:
one container uses another. It never goes to a provider.

### Region one, the top: DAEMON ALPHA

A torn label in the corner reads `DAEMON α`, the alpha drawn large in
red marker and circled twice.

Three cabinets in a row, equal in size, left to right:

1. Agent `RED HERRING`, eye stamp. **Amber glow** (`I`).
2. Agent `DEAD DROP`, eye stamp. **Violet glow** (`III`).
3. Tool `THE ARCHIVE`, gear stamp. **Cyan glow** (`II`).

Under `RED HERRING`, three tags on short red strings:

| Tag | Glyph scratched on it | Fragment | Glow |
|---|---|---|---|
| the scraper | a magnifying glass over a torn strip of newsprint | `3f9a` | violet (`III`) |
| the drawer | a single filing-cabinet drawer, pulled half open | `b17c` | cyan (`II`) |
| the cipher | a cipher wheel, two rings of letters, one turned | `e02d` | violet (`III`) |

Under `DEAD DROP`, two tags on short red strings:

| Tag | Glyph scratched on it | Fragment | Glow |
|---|---|---|---|
| the courier | an envelope with a wax seal, the seal cracked | `77a1` | amber (`I`) |
| the ledger | a ledger book, open, two columns of tally marks | `c5f0` | cyan (`II`) |

`DEAD DROP` is also attached to `THE ARCHIVE`: a thicker red string
between the two cabinets, a small paper tag at its midpoint reading
only `attached`.

### Region two, the middle: DAEMON BETA

A torn label in the corner reads `DAEMON β`, the beta drawn large in
red marker and circled the same way.

One cabinet: agent `CASSANDRA`, eye stamp, the same size as the three
above. **Amber glow** (`I`). Under her, three tags on short red
strings:

| Tag | Glyph scratched on it | Fragment | Glow |
|---|---|---|---|
| the scanner | a dial radio, needle on the band, a lightning mark beside it | `a9e4` | violet (`III`) |
| the map | a folded road map with three pinholes in it | `0b6c` | cyan (`II`) |
| the stopwatch | a stopwatch, the hand at twelve | `d18f` | cyan (`II`) |

`CASSANDRA` is also joined to `THE ARCHIVE` up in region one: a long
red string climbs from her cabinet across the seam into the first
region and ends at `THE ARCHIVE`. Where it crosses the seam a scrap
of newsprint over it reads only `exposed`. It is the one line that
crosses between the daemons, and it is what the poster is about.

### Region three, the bottom: PROVIDERS

A torn label in the corner reads `PROVIDERS`. The three lit
photographs, as the table above states them, amber on the left, cyan
in the middle, violet on the right, their glows bleeding up into the
paper toward the things they power.

### The tally, so the colours are right

| Glow | Provider | Lit by it |
|---|---|---|
| amber | `I`, the transformer | `RED HERRING`, the courier `77a1`, `CASSANDRA` |
| cyan | `II`, the servers | the drawer `b17c`, the ledger `c5f0`, `THE ARCHIVE`, the map `0b6c`, the stopwatch `d18f` |
| violet | `III`, the mast | the scraper `3f9a`, the cipher `e02d`, `DEAD DROP`, the scanner `a9e4` |

Twelve things, twelve glows, one colour each. No agent shares a
colour with any of its own tags. The two agents of daemon alpha are
different colours. `THE ARCHIVE` is cyan, which neither `DEAD DROP`
nor `CASSANDRA` is. The scatter is the point: a daemon is a region,
not a colour.

### Everything else

- No other words appear: the sentence at the top, the three region
  labels, the four codenames, the eight hex fragments, the two scraps
  on strings, the three numerals, and that is all.
- Texture is free: creases, staple marks, a torn corner, over-spray,
  the grain of a copy of a copy. The glows are soft and the stencils
  are hard.
- Nothing is explained on the poster. Whoever it is for already
  knows.

## The cast, against the system

What each thing in the picture is, so the picture can be checked
against the code.

- **`RED HERRING`, `DEAD DROP`** — two agents of daemon alpha, each
  made from an agent template by `agents::create`, running on one
  provider each. **`CASSANDRA`** — one agent of daemon beta, the same.
- **`THE ARCHIVE`** — a tool of daemon alpha on record, made from a
  tool template by `tools::create`; daemon alpha runs its container.
- **The eight tags** — dependency tools: each is a
  `shared::containers::dependencies::Template` the agent's program
  declared at register time, deployed by its daemon then and there
  on any connected provider, alive for the agent's life, named by
  its template id (the fragment is the id's first characters) and by
  nothing else. The glyph is what the tool is for; the system never
  records a name for one:

  | Tag | What it is | Whose |
  |---|---|---|
  | the scraper `3f9a` | fetches and strips pages | `RED HERRING` |
  | the drawer `b17c` | a files store the agent reads and writes | `RED HERRING` |
  | the cipher `e02d` | encodes and decodes messages | `RED HERRING` |
  | the courier `77a1` | sends and receives sealed messages | `DEAD DROP` |
  | the ledger `c5f0` | a database the agent keeps accounts in | `DEAD DROP` |
  | the scanner `a9e4` | listens to feeds and raises alarms | `CASSANDRA` |
  | the map `0b6c` | geocodes and plots places | `CASSANDRA` |
  | the stopwatch `d18f` | schedules and times what the agent does | `CASSANDRA` |

- **`attached`** — `tools::attach`: `DEAD DROP` is served `THE
  ARCHIVE` among its MCP servers; the tool's container is started
  when `DEAD DROP` becomes active and held while it is.
- **`exposed`** — daemon beta holds a `providers::daemons` record for
  daemon alpha; `CASSANDRA` is attached to a connected tool that
  names that record and `THE ARCHIVE`. While `CASSANDRA` is active,
  daemon beta connects to daemon alpha through a provider both are
  connected to, opens `tools::expose` there, and joins the container
  the expose answers. Daemon beta never runs `THE ARCHIVE` and never
  sees its image; it holds a line to it.
- **The glows** — every container, agent, tool or dependency, runs
  on exactly one provider, chosen container by container among the
  connected providers in random order, never daemon by daemon. A
  daemon has no provider of its own; a daemon is a region, the space
  its containers stand in, and the light falls across it from
  wherever each container happens to run.
