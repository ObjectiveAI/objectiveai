# Image prompt: two daemons, one tool between them, three providers underneath

An image prompt for one picture of what landed on 2026-10-10: a tool
one daemon runs, joined by an agent of another daemon through a
provider, every container of either daemon powered by one of three
providers. The picture is a wall, not a diagram: the system drawn the
way a conspiracy theorist pins it to a corkboard, and the way an
anarchist stencils it on a shutter. Almost no text. The structure
carries the meaning.

Every entity in the picture is named in §The cast, every dependency
tool is defined with the glyph drawn on it, and every string to a
provider is stated in §The strings. Nothing is left for the
illustrator to decide about who connects to what.

## Prompt

A tall poster, portrait, in a hand-made anarchist / conspiracy-theory
style: xeroxed zine texture, black spray-paint stencil edges, red
string pinned between things with real pins, torn newsprint scraps,
smudged photocopier grain, a corkboard-and-concrete background, white
chalk and red marker over black. Collage, not vector. Nothing is
clean; everything is deliberate. No people, no faces, no logos.
Lettering is stencil or scrawled marker, and there is very little of
it.

**Across the very top, the only sentence in the picture, stencilled
in white spray paint, slightly misregistered: `DO NOT BE AFRAID.`**

The poster is cut into three horizontal bands by two ragged strips of
black duct tape running edge to edge.

### The two shapes

- **A cabinet.** Every agent and every tool is a square photograph
  of a locked steel cabinet, pasted down, the same size, with its
  codename stencilled across the door. An agent's door is stamped
  with a stencilled **eye**; a tool's door is stamped with a
  stencilled **gear**. That stamp is the only thing that tells an
  agent from a tool.
- **A tag.** Every dependency tool is a luggage tag, a third the size
  of a cabinet, hanging under its agent on a short red string. A tag
  carries two things and nothing else: a **glyph** scratched into it
  in black marker, stated per tag below, and a four-character hex
  fragment in tiny monospace in one corner. No tag has a name.

### Band one, the top: DAEMON ALPHA

A torn label in the band's upper-left corner reads `DAEMON α`, the
word stencilled and the alpha drawn large in red marker, circled
twice as if it mattered.

Three cabinets in a row, left to right, equal in size:

1. Agent `RED HERRING`, eye stamp.
2. Agent `DEAD DROP`, eye stamp.
3. Tool `THE ARCHIVE`, gear stamp.

Under `RED HERRING`, three tags on three short strings, left to right:

| Tag | Glyph scratched on it | Fragment |
|---|---|---|
| the scraper | a magnifying glass over a torn strip of newsprint | `3f9a` |
| the drawer | a single filing-cabinet drawer, pulled half open | `b17c` |
| the cipher | a cipher wheel, two rings of letters, one turned | `e02d` |

Under `DEAD DROP`, two tags on two short strings, left to right:

| Tag | Glyph scratched on it | Fragment |
|---|---|---|
| the courier | an envelope with a wax seal, the seal cracked | `77a1` |
| the ledger | a ledger book, open, two columns of tally marks | `c5f0` |

`DEAD DROP` is also attached to `THE ARCHIVE`: a thicker red string
pinned at both cabinets, with a small paper tag hanging from its
midpoint that reads only `attached`.

### Band two, the middle: DAEMON BETA

A torn label in the band's upper-left corner reads `DAEMON β`, the
beta drawn large in red marker and circled the same way.

One cabinet: agent `CASSANDRA`, eye stamp, the same size as the three
above. Under it, three tags on three short strings, left to right:

| Tag | Glyph scratched on it | Fragment |
|---|---|---|
| the scanner | a dial radio, needle on the band, a lightning mark beside it | `a9e4` |
| the map | a folded road map with three pinholes in it | `0b6c` |
| the stopwatch | a stopwatch, the hand at twelve | `d18f` |

`CASSANDRA` is also joined to `THE ARCHIVE` in band one: a long red
string climbs from her cabinet up through the duct tape into the
first band and pins into `THE ARCHIVE`'s cabinet. Where the string
crosses the tape a small scrap of newsprint is pinned over it reading
only `exposed`. This string is the one line in the picture that
crosses a band border between the daemons, and it is the thing the
poster is about.

### Band three, the bottom: PROVIDERS

A torn label in the band's upper-left corner reads `PROVIDERS`.

Three providers across the bottom of the poster, left to right, each
a grainy black-and-white photograph pasted down and stamped with a
stencilled numeral and nothing else:

| Position | Photograph | Stamp |
|---|---|---|
| left | a substation transformer, insulators and cooling fins | `I` |
| middle | a bank of rack servers, cable spaghetti, one lit LED | `II` |
| right | a radio mast against a flat sky, guy wires | `III` |

### The strings to the providers

Twelve entities stand in the two bands above: in band one, two
agents, one tool and five tags; in band two, one agent and three
tags. From each of the twelve, exactly one red string runs down into
the third band and pins into exactly one provider. Twelve strings,
twelve pins. No entity has two strings; none has none. The
assignment is exactly this:

| Entity | Band | Provider |
|---|---|---|
| `RED HERRING` (agent) | α | `I`, the transformer |
| the scraper `3f9a` | α | `III`, the mast |
| the drawer `b17c` | α | `II`, the servers |
| the cipher `e02d` | α | `III`, the mast |
| `DEAD DROP` (agent) | α | `III`, the mast |
| the courier `77a1` | α | `I`, the transformer |
| the ledger `c5f0` | α | `II`, the servers |
| `THE ARCHIVE` (tool) | α | `II`, the servers |
| `CASSANDRA` (agent) | β | `I`, the transformer |
| the scanner `a9e4` | β | `III`, the mast |
| the map `0b6c` | β | `II`, the servers |
| the stopwatch `d18f` | β | `II`, the servers |

So the transformer `I` takes three strings (`RED HERRING`, the
courier, `CASSANDRA`); the servers `II` take five (the drawer, the
ledger, `THE ARCHIVE`, the map, the stopwatch); the mast `III` takes
four (the scraper, the cipher, `DEAD DROP`, the scanner). The pins
cluster where the strings arrive.

The scramble is the point, and it is built into the table: no agent
shares a provider with any of its own tags; the two agents of daemon
alpha go to different providers; `THE ARCHIVE` goes to the servers,
which neither `DEAD DROP` nor `CASSANDRA` goes to; the scraper, the
leftmost tag on the poster, runs all the way to the mast on the far
right; the courier runs from the middle of band one back to the
transformer on the far left; the three tags under `CASSANDRA` fan
out to all of `III`, `II`, `II` while she herself goes to `I`. The
twelve strings cross and recross in a dense red web over the lower
half of the poster. Chaotic to look at, simple to trace: each has one
start and one end.

### Everything else

- All strings are the same red: the tag strings short, the provider
  strings long, the `attached` string thicker, the `exposed` string
  the longest. Nothing else connects anything.
- Pins are real push-pins, drawn with a shadow.
- Under a few cabinets a scrap of masking tape carries a scrawled
  marker note too small to read, as if someone annotated the wall
  and did not want it read from a distance.
- Texture everywhere: fold creases, a coffee ring on the second
  band, staple marks at the corners, a corner torn off and left
  hanging.
- No other words appear. No legend, no title besides the sentence
  at the top, no arrows with labels. The eye stamp, the gear stamp,
  the three band labels, the four codenames, the eight hex
  fragments, the two tags on strings, and the three numerals are
  the whole vocabulary.

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
- **The twelve provider strings** — every container, agent, tool or
  dependency, runs on exactly one provider, chosen container by
  container among the connected providers in random order, never
  daemon by daemon. A daemon has no provider of its own; a daemon is
  a band, the space its containers stand in.
